//! Typed client preferences stored with audio in the existing settings document.

use crate::{
    client_shell::ClientSettings,
    core_adapter::CorePreferences,
    visuals::{HealthDisplay, HealthMode},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StoredHealthMode {
    Selected,
    DamagedOrSelected,
    Always,
}

/// Storage representation; runtime owners remain the existing settings resources.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ControlSettings {
    pub scroll_speed: f32,
    pub edge_scroll: bool,
    pub ui_scale: f32,
    pub cursor_lock: bool,
    pub double_click_ms: u64,
    pub double_click_pixels: f32,
    pub health_mode: StoredHealthMode,
    /// The player setting "frame limit" when the window has the focus (`pacing.rs`).
    pub frame_limit: u32,
    /// The player setting "frame limit" when the window does not have the focus.
    pub background_frame_limit: u32,
}

impl Default for ControlSettings {
    fn default() -> Self {
        let settings = ClientSettings::default();
        Self {
            scroll_speed: settings.scroll_speed,
            edge_scroll: settings.edge_scroll,
            ui_scale: settings.ui_scale,
            cursor_lock: settings.cursor_lock,
            double_click_ms: settings.double_click_ms,
            double_click_pixels: turts_interaction::InteractionSettings::default()
                .double_click_pixels,
            health_mode: StoredHealthMode::DamagedOrSelected,
            frame_limit: settings.frame_limit,
            background_frame_limit: settings.background_frame_limit,
        }
    }
}

impl ControlSettings {
    pub(crate) fn valid(&self) -> bool {
        self.scroll_speed.is_finite()
            && (0.25..=3.0).contains(&self.scroll_speed)
            && self.ui_scale.is_finite()
            && (0.75..=2.0).contains(&self.ui_scale)
            && (100..=1000).contains(&self.double_click_ms)
            && self.double_click_pixels.is_finite()
            && self.double_click_pixels >= 0.0
            && crate::pacing::valid_frame_limit(self.frame_limit)
            && crate::pacing::valid_frame_limit(self.background_frame_limit)
    }

    pub(crate) fn apply(
        &self,
        settings: Option<&mut ClientSettings>,
        health: Option<&mut HealthDisplay>,
        preferences: Option<&mut CorePreferences>,
    ) {
        if let Some(settings) = settings {
            *settings = ClientSettings {
                scroll_speed: self.scroll_speed,
                edge_scroll: self.edge_scroll,
                ui_scale: self.ui_scale,
                cursor_lock: self.cursor_lock,
                double_click_ms: self.double_click_ms,
                frame_limit: self.frame_limit,
                background_frame_limit: self.background_frame_limit,
            };
        }
        if let Some(health) = health {
            health.0 = match self.health_mode {
                StoredHealthMode::Selected => HealthMode::Selected,
                StoredHealthMode::DamagedOrSelected => HealthMode::DamagedOrSelected,
                StoredHealthMode::Always => HealthMode::Always,
            };
        }
        if let Some(preferences) = preferences {
            preferences.interaction.double_click_ms = self.double_click_ms;
            preferences.interaction.double_click_pixels = self.double_click_pixels;
        }
    }

    pub(crate) fn capture(
        settings: Option<&ClientSettings>,
        health: Option<&HealthDisplay>,
        preferences: Option<&CorePreferences>,
        fallback: Option<Self>,
    ) -> Self {
        let mut value = fallback.unwrap_or_default();
        if let Some(settings) = settings {
            value.scroll_speed = settings.scroll_speed;
            value.edge_scroll = settings.edge_scroll;
            value.ui_scale = settings.ui_scale;
            value.cursor_lock = settings.cursor_lock;
            value.double_click_ms = settings.double_click_ms;
            value.frame_limit = settings.frame_limit;
            value.background_frame_limit = settings.background_frame_limit;
        }
        if let Some(health) = health {
            value.health_mode = match health.0 {
                HealthMode::Selected => StoredHealthMode::Selected,
                HealthMode::DamagedOrSelected => StoredHealthMode::DamagedOrSelected,
                HealthMode::Always => StoredHealthMode::Always,
            };
        }
        if let Some(preferences) = preferences {
            value.double_click_pixels = preferences.interaction.double_click_pixels;
        }
        value
    }
}

#[cfg(test)]
mod frame_limit_tests {
    use super::ControlSettings;
    use crate::volume_settings::{decode_settings_json, set_controls_in_json};

    #[test]
    fn a_stored_frame_limit_loads_and_saves() {
        let loaded = decode_settings_json(r#"{"controls":{"frame_limit":30}}"#);
        assert_eq!(loaded.notice, None);
        assert_eq!(loaded.controls.frame_limit, 30);
        assert_eq!(loaded.controls.background_frame_limit, 60);
        let encoded = set_controls_in_json(None, &loaded.controls).unwrap();
        assert_eq!(decode_settings_json(&encoded).controls, loaded.controls);
    }

    #[test]
    fn a_document_without_the_frame_limit_keeps_the_defaults() {
        let loaded = decode_settings_json(r#"{"controls":{"scroll_speed":1.5}}"#);
        assert_eq!(loaded.notice, None);
        assert_eq!(loaded.controls.frame_limit, 0);
        assert_eq!(loaded.controls.background_frame_limit, 60);
    }

    #[test]
    fn an_invalid_stored_frame_limit_falls_back_to_the_defaults_with_the_notice() {
        for json in [
            r#"{"controls":{"frame_limit":5}}"#,
            r#"{"controls":{"frame_limit":241}}"#,
            r#"{"controls":{"background_frame_limit":9}}"#,
            r#"{"controls":{"frame_limit":-1}}"#,
            r#"{"controls":{"frame_limit":"30"}}"#,
        ] {
            let loaded = decode_settings_json(json);
            assert!(loaded.notice.is_some(), "{json} must give the notice");
            assert_eq!(loaded.controls, ControlSettings::default(), "{json}");
        }
        let invalid = ControlSettings {
            frame_limit: 5,
            ..ControlSettings::default()
        };
        assert!(set_controls_in_json(None, &invalid).is_err());
    }
}
