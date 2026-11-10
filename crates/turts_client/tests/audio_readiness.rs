use turts_client::{
    audio::{browser_contexts_ready, set_sink_gain},
    volume_settings::{VolumeCategory, VolumeSettings},
};

#[test]
fn no_player_is_created_without_real_contexts_or_while_any_are_suspended() {
    assert!(
        !browser_contexts_ready(&[], true),
        "missing context observation stays locked"
    );
    assert!(
        !browser_contexts_ready(&["running".into(), "suspended".into()], true),
        "one suspended context keeps all new AudioPlayer entities gated"
    );
    assert!(
        !browser_contexts_ready(&["closed".into()], true),
        "a closed context does not count as resumed"
    );
}

#[test]
fn all_actual_contexts_must_report_running_before_audio_can_start() {
    assert!(
        !browser_contexts_ready(&["running".into()], false),
        "an autoplay-running context does not prove real player activation"
    );
    assert!(browser_contexts_ready(&["running".into()], true));
    assert!(browser_contexts_ready(
        &["running".into(), "running".into()],
        true
    ));
}

#[test]
fn volume_changes_update_existing_sinks_with_master_times_category_gain() {
    let settings = VolumeSettings {
        master: 0.9,
        voice: 0.8,
        alerts: 0.7,
        effects: 0.6,
    };
    let mut voice_sink_gain = 1.0;
    let mut alert_sink_gain = 1.0;
    set_sink_gain(settings, VolumeCategory::Voice, |gain| {
        voice_sink_gain = gain
    });
    set_sink_gain(settings, VolumeCategory::Alerts, |gain| {
        alert_sink_gain = gain
    });
    assert!(
        (voice_sink_gain - 0.72).abs() < f32::EPSILON,
        "live voice sinks receive master multiplied by voice volume"
    );
    assert!(
        (alert_sink_gain - 0.63).abs() < f32::EPSILON,
        "live alert sinks receive master multiplied by alert volume"
    );
}
