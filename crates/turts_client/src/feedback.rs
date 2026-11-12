//! Local feedback derived from the coherent owner replica and authored interface rules.

use crate::{
    core_adapter::{CoreAdapterSet, CoreRuntime},
    debug_feed::{AlertPingsFeed, AlertsFeed, AudioFeed, NoticesFeed, SelectionFeed, StartedSound},
    ingest::{CommandResult, CommandResults, ReplicaState},
    join::{AcquiredPackages, Phase},
    replica::Replica,
    volume_settings::VolumeSettings,
};
use bevy::prelude::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::AlertTrigger;
use turts_net_protocol::{
    CommandRejectReason, ExtractorStatus, GatherStage, InserterStage, JobStage, WeaponStatus,
};

#[derive(Resource, Debug, Default, Clone, PartialEq, Serialize)]
pub struct FrameFeedback {
    pub audio_started: Vec<StartedSound>,
    pub notices: Vec<serde_json::Value>,
    pub alerts: Vec<serde_json::Value>,
}

#[derive(Resource, Default)]
struct FeedbackRuntime {
    previous: Option<Replica>,
    no_orders_notice_sent: bool,
    result_cursor: usize,
    cooldowns: BTreeMap<String, f64>,
    wait_started: BTreeMap<(String, u32), u32>,
    next_ping_id: u64,
    previous_selection: Vec<u32>,
    joined_match: Option<u64>,
}

/// The `wait_started` tick of a wait that has already alerted. No tick is later, so the wait
/// does not pass its threshold again until it ends and a new wait starts.
const WAIT_ALERTED: u32 = u32::MAX;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeedbackSet;

pub struct FeedbackPlugin;
impl Plugin for FeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameFeedback>()
            .init_resource::<FeedbackRuntime>()
            .init_resource::<NoticesFeed>()
            .init_resource::<AlertsFeed>()
            .init_resource::<AlertPingsFeed>()
            .add_systems(
                Update,
                update_feedback.after(CoreAdapterSet).in_set(FeedbackSet),
            );
    }
}

pub fn reason_text(reason: CommandRejectReason, overrides: &BTreeMap<String, String>) -> String {
    if let Some(text) = overrides
        .get(reason.as_str())
        .filter(|text| !text.trim().is_empty())
    {
        return text.clone();
    }
    match reason {
        CommandRejectReason::EmptyCommand => "No order was selected".into(),
        CommandRejectReason::TooManyUnits => "Too many units".into(),
        CommandRejectReason::DuplicateUnit => "A unit was listed more than once".into(),
        CommandRejectReason::UnknownUnit => "Unit no longer exists".into(),
        CommandRejectReason::NotOwner => "Not your unit".into(),
        CommandRejectReason::NotMobile => "This unit cannot move".into(),
        CommandRejectReason::TargetNotFinite => "Target position is invalid".into(),
        CommandRejectReason::TargetOutsideMap => "Target is outside the map".into(),
        CommandRejectReason::MissingCapability => "This unit cannot do that".into(),
        CommandRejectReason::UnknownNode => "Resource node no longer exists".into(),
        CommandRejectReason::IncompatibleItem => "This item is not accepted here".into(),
        CommandRejectReason::NodeDepleted => "Resource node is depleted".into(),
        CommandRejectReason::UnknownRecipe => "Recipe is unavailable".into(),
        CommandRejectReason::RecipeNotEnabled => "Recipe is not enabled".into(),
        CommandRejectReason::QueueFull => "The order queue is full".into(),
        CommandRejectReason::InsufficientBalance => "Not enough resources".into(),
        CommandRejectReason::InsufficientItems => "Not enough items".into(),
        CommandRejectReason::UnknownJob => "Job no longer exists".into(),
        CommandRejectReason::RefundDestinationFull => "There is no room for the refund".into(),
        CommandRejectReason::RefundDestinationOverflow => {
            "The refund exceeds available storage".into()
        }
        CommandRejectReason::ArithmeticOverflow => "The requested amount is too large".into(),
        CommandRejectReason::IdExhausted => "The game cannot assign another identifier".into(),
        CommandRejectReason::BadSequence => "Order sequence is invalid".into(),
        CommandRejectReason::NotRemovable => "This unit cannot be removed".into(),
        CommandRejectReason::NotToggleable => "This unit cannot be toggled".into(),
        CommandRejectReason::PrerequisitesMissing => "Research prerequisites are missing".into(),
        CommandRejectReason::AlreadyResearched => "This technology is already researched".into(),
        CommandRejectReason::ResearchInProgress => "Research is already in progress".into(),
        CommandRejectReason::Internal => "The order could not be completed".into(),
        CommandRejectReason::UnknownOrder => "That order is unavailable".into(),
        CommandRejectReason::NotApplicable => "Cannot do that".into(),
        CommandRejectReason::TargetInvalid => "That is not a valid target".into(),
        CommandRejectReason::TargetGone => "That target is gone".into(),
        CommandRejectReason::OutOfRange => "Target out of range".into(),
        CommandRejectReason::RateLimited => "Too many orders".into(),
        CommandRejectReason::NotQueueable => "This order cannot be queued".into(),
        CommandRejectReason::Blocked => "The unit cannot reach that place".into(),
    }
}

pub fn format_notice(unit_name: &str, reason_text: &str, count: usize) -> String {
    if count <= 1 {
        format!("{unit_name}: {reason_text}")
    } else {
        format!("{unit_name} (×{count}): {reason_text}")
    }
}

// Keep each optional Bevy resource explicit: the feedback plugin also runs in
// partial client harnesses where some app resources are intentionally absent.
#[allow(clippy::too_many_arguments)]
fn update_feedback(
    time: Res<Time>,
    state: Option<Res<ReplicaState>>,
    phase: Option<Res<Phase>>,
    acquired: Option<Res<AcquiredPackages>>,
    results: Option<Res<CommandResults>>,
    mut runtime: ResMut<FeedbackRuntime>,
    mut frame: ResMut<FrameFeedback>,
    mut notices: ResMut<NoticesFeed>,
    mut alert_feed: ResMut<AlertsFeed>,
    mut pings: ResMut<AlertPingsFeed>,
    mut audio_feed: ResMut<AudioFeed>,
    settings: Option<Res<VolumeSettings>>,
    selection: Option<Res<SelectionFeed>>,
    core: Option<Res<CoreRuntime>>,
) {
    frame.notices.clear();
    frame.alerts.clear();
    frame.audio_started.clear();
    let now = time.elapsed_secs_f64();
    if let (Some(state), Some(acquired)) = (state.as_deref(), acquired.as_deref()) {
        if let Some(welcome) = state.welcome
            && runtime.joined_match != Some(welcome.match_id)
        {
            runtime.joined_match = Some(welcome.match_id);
            runtime.previous = None;
            runtime.no_orders_notice_sent = false;
            runtime.cooldowns.clear();
            runtime.wait_started.clear();
            runtime.previous_selection.clear();
            runtime.result_cursor = results.as_deref().map_or(0, |results| results.0.len());
        }
        if phase
            .as_deref()
            .is_some_and(|phase| matches!(phase, Phase::Joined { .. }))
            && state.welcome.is_some()
            && acquired.0.content.catalog.orders.is_empty()
            && !runtime.no_orders_notice_sent
        {
            push_notice("This game defines no orders".into(), &mut frame);
            runtime.no_orders_notice_sent = true;
        }
        let empty_overrides = BTreeMap::new();
        let overrides = acquired
            .0
            .presentation
            .interface
            .as_ref()
            .map(|interface| &interface.reasons)
            .unwrap_or(&empty_overrides);
        if let Some(results) = results.as_deref() {
            for result in results.0.iter().skip(runtime.result_cursor) {
                if let CommandResult::Order {
                    refused, envelope, ..
                } = result
                {
                    if let Some(interface) = acquired.0.presentation.interface.as_ref()
                        && (!refused.is_empty() || envelope.is_some())
                        && let Some(sound) = refused
                            .first()
                            .and_then(|refusal| {
                                state
                                    .replica
                                    .as_ref()
                                    .and_then(|replica| replica.units.get(&refusal.unit.0))
                                    .and_then(|unit| interface.units.get(&unit.content_id))
                                    .and_then(|unit| unit.responses.get("error"))
                                    .and_then(|sounds| sounds.first())
                            })
                            .or_else(|| {
                                interface.units.values().find_map(|unit| {
                                    unit.responses
                                        .get("error")
                                        .and_then(|sounds| sounds.first())
                                })
                            })
                    {
                        frame.audio_started.push(StartedSound {
                            id: qualify_sound(&acquired.0.presentation.interface_package, sound),
                            category: "effects".into(),
                            priority: 0,
                        });
                    }
                    let mut grouped = Vec::<(String, CommandRejectReason, String, usize)>::new();
                    for refusal in refused {
                        let key = state
                            .replica
                            .as_ref()
                            .and_then(|replica| replica.units.get(&refusal.unit.0))
                            .map(|unit| unit.content_id.clone())
                            .unwrap_or_else(|| "Unit".into());
                        let name = acquired
                            .0
                            .unit_names
                            .get(&key)
                            .cloned()
                            .unwrap_or_else(|| display_content_name(&key));
                        if let Some(entry) =
                            grouped.iter_mut().find(|(content_id, reason, _, _)| {
                                content_id == &key && *reason == refusal.reason
                            })
                        {
                            entry.3 += 1;
                        } else {
                            grouped.push((key, refusal.reason, name, 1));
                        }
                    }
                    for (_, reason, name, count) in grouped {
                        let text = reason_text(reason, overrides);
                        push_notice(format_notice(&name, &text, count), &mut frame);
                    }
                    if let Some(reason) = envelope {
                        push_notice(reason_text(*reason, overrides), &mut frame);
                    }
                } else if matches!(result, CommandResult::Rejected { .. }) {
                    if let CommandResult::Rejected { reason, .. } = result {
                        push_notice(reason_text(*reason, overrides), &mut frame);
                    }
                    if let Some(sound) =
                        acquired
                            .0
                            .presentation
                            .interface
                            .as_ref()
                            .and_then(|interface| {
                                interface.units.values().find_map(|unit| {
                                    unit.responses
                                        .get("error")
                                        .and_then(|sounds| sounds.first())
                                })
                            })
                    {
                        frame.audio_started.push(StartedSound {
                            id: qualify_sound(&acquired.0.presentation.interface_package, sound),
                            category: "effects".into(),
                            priority: 0,
                        });
                    }
                }
            }
            runtime.result_cursor = results.0.len();
        }
        if let (Some(replica), Some(previous), Some(interface)) = (
            state.replica.as_ref(),
            runtime.previous.as_ref(),
            acquired.0.presentation.interface.as_ref(),
        ) {
            let mut failures = Vec::<(String, CommandRejectReason, usize)>::new();
            let mut failure_sound_started = false;
            for (&unit_id, order) in &replica.unit_orders {
                let Some(failure) = order.last_failure.as_ref() else {
                    continue;
                };
                let unchanged = previous
                    .unit_orders
                    .get(&unit_id)
                    .and_then(|old| old.last_failure.as_ref())
                    == Some(failure);
                if unchanged {
                    continue;
                }
                let Some(unit) = replica.units.get(&unit_id) else {
                    continue;
                };
                if let Some(entry) = failures.iter_mut().find(|(content_id, reason, _)| {
                    content_id == &unit.content_id && *reason == failure.reason
                }) {
                    entry.2 += 1;
                } else {
                    failures.push((unit.content_id.clone(), failure.reason, 1));
                }
                if !failure_sound_started
                    && let Some(sound) = interface
                        .units
                        .get(&unit.content_id)
                        .and_then(|presentation| presentation.responses.get("error"))
                        .and_then(|sounds| sounds.first())
                {
                    frame.audio_started.push(StartedSound {
                        id: qualify_sound(&acquired.0.presentation.interface_package, sound),
                        category: "effects".into(),
                        priority: 0,
                    });
                    failure_sound_started = true;
                }
            }
            for (content_id, reason, count) in failures {
                let name = acquired
                    .0
                    .unit_names
                    .get(&content_id)
                    .cloned()
                    .unwrap_or_else(|| display_content_name(&content_id));
                push_notice(
                    format_notice(&name, &reason_text(reason, overrides), count),
                    &mut frame,
                );
            }
        }
        if let (Some(selection), Some(replica), Some(interface)) = (
            selection.as_deref(),
            state.replica.as_ref(),
            acquired.0.presentation.interface.as_ref(),
        ) && selection.0 != runtime.previous_selection
        {
            for id in selection
                .0
                .iter()
                .filter(|id| !runtime.previous_selection.contains(id))
            {
                let Some(unit) = replica.units.get(id) else {
                    continue;
                };
                if unit.owner_slot != state.welcome.map(|welcome| welcome.slot).unwrap_or(0) {
                    continue;
                }
                if let Some(sound) = interface
                    .units
                    .get(&unit.content_id)
                    .and_then(|presentation| presentation.responses.get("select"))
                    .and_then(|sounds| sounds.first())
                {
                    frame.audio_started.push(StartedSound {
                        id: qualify_sound(&acquired.0.presentation.interface_package, sound),
                        category: "voice".into(),
                        priority: 0,
                    });
                    break;
                }
            }
            runtime.previous_selection.clone_from(&selection.0);
        }
        if let Some(interface) = acquired.0.presentation.interface.as_ref()
            && let Some(replica) = state.replica.as_ref()
        {
            let previous = runtime.previous.take();
            let events = collect_alert_events(
                replica,
                previous.as_ref(),
                state.welcome.map(|welcome| welcome.slot).unwrap_or(0),
                &acquired.0.content,
                interface.alerts.as_slice(),
                now,
                &mut runtime,
            );
            for event in events {
                frame.alerts.push(serde_json::json!({"id":event.rule_id,"text":event.text,"unit":event.unit,"position":event.position}));
                alert_feed.0.push(serde_json::json!({"id":event.rule_id,"text":event.text,"message":event.text,"unit":event.unit,"position":event.position}));
                if let Some(position) = event.position
                    && event.minimap_ping
                {
                    runtime.next_ping_id = runtime.next_ping_id.saturating_add(1);
                    pings.0.push(serde_json::json!({
                        "id":runtime.next_ping_id,
                        "alert_id":&event.rule_id,
                        "tick":replica.last_applied_tick,
                        "unit":event.unit,
                        "x":position.x,
                        "y":position.y,
                        "text":&event.text,
                    }));
                }
                if let Some(sound) = event.sound {
                    frame.audio_started.push(StartedSound {
                        id: qualify_sound(&acquired.0.presentation.interface_package, &sound),
                        category: "alerts".into(),
                        priority: event.priority,
                    });
                }
            }
            while alert_feed.0.len() > 5 {
                alert_feed.0.remove(0);
            }
            while pings.0.len() > 64 {
                pings.0.remove(0);
            }
            runtime.previous = Some(replica.clone());
        }
        if let Some(core) = core.as_deref() {
            for effect in core.effects() {
                if effect.kind == "order_intent" {
                    let Some(interface) = acquired.0.presentation.interface.as_ref() else {
                        continue;
                    };
                    let Some(units) = effect
                        .value
                        .get("units")
                        .and_then(serde_json::Value::as_array)
                    else {
                        continue;
                    };
                    for id in units
                        .iter()
                        .filter_map(serde_json::Value::as_u64)
                        .map(|id| id as u32)
                    {
                        let Some(unit) = state
                            .replica
                            .as_ref()
                            .and_then(|replica| replica.units.get(&id))
                        else {
                            continue;
                        };
                        if let Some(sound) = interface
                            .units
                            .get(&unit.content_id)
                            .and_then(|presentation| presentation.responses.get("order"))
                            .and_then(|sounds| sounds.first())
                        {
                            frame.audio_started.push(StartedSound {
                                id: qualify_sound(
                                    &acquired.0.presentation.interface_package,
                                    sound,
                                ),
                                category: "voice".into(),
                                priority: 0,
                            });
                            break;
                        }
                    }
                } else if effect.kind == "sound" {
                    let Some(id) = effect.value.get("id").and_then(serde_json::Value::as_str)
                    else {
                        continue;
                    };
                    let category = effect
                        .value
                        .get("category")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("effects");
                    let category = match category {
                        "select" | "selection" | "order" | "orders" => "voice",
                        "alert" => "alerts",
                        _ => "effects",
                    };
                    let sound_id = qualify_sound(&acquired.0.presentation.interface_package, id);
                    if !frame
                        .audio_started
                        .iter()
                        .any(|sound| sound.id == sound_id && sound.category == category)
                    {
                        frame.audio_started.push(StartedSound {
                            id: sound_id,
                            category: category.into(),
                            priority: 0,
                        });
                    }
                } else if effect.kind == "notice" {
                    let key = effect
                        .value
                        .get("text_key")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    let Some(unit_ids) = effect
                        .value
                        .get("units")
                        .and_then(serde_json::Value::as_array)
                    else {
                        continue;
                    };
                    let fallback = if key == "no_applicable_order" {
                        "Cannot do that"
                    } else {
                        "Order unavailable"
                    };
                    let text = acquired
                        .0
                        .presentation
                        .interface
                        .as_ref()
                        .and_then(|interface| {
                            interface.reasons.get(key).or_else(|| {
                                (key == "no_applicable_order")
                                    .then(|| interface.reasons.get("not_applicable"))
                                    .flatten()
                            })
                        })
                        .map(String::as_str)
                        .unwrap_or(fallback);
                    let mut names = BTreeMap::<String, usize>::new();
                    for id in unit_ids
                        .iter()
                        .filter_map(serde_json::Value::as_u64)
                        .map(|id| id as u32)
                    {
                        let content_id = state
                            .replica
                            .as_ref()
                            .and_then(|replica| replica.units.get(&id))
                            .map(|unit| unit.content_id.as_str())
                            .unwrap_or("Unit");
                        let name = acquired
                            .0
                            .unit_names
                            .get(content_id)
                            .cloned()
                            .unwrap_or_else(|| display_content_name(content_id));
                        *names.entry(name).or_default() += 1;
                    }
                    for (name, count) in names {
                        push_notice(format_notice(&name, text, count), &mut frame);
                    }
                    if let Some(interface) = acquired.0.presentation.interface.as_ref() {
                        let unit_id = unit_ids
                            .first()
                            .and_then(serde_json::Value::as_u64)
                            .map(|id| id as u32);
                        let sound = unit_id
                            .and_then(|id| {
                                state
                                    .replica
                                    .as_ref()
                                    .and_then(|replica| replica.units.get(&id))
                            })
                            .and_then(|unit| interface.units.get(&unit.content_id))
                            .and_then(|unit| unit.responses.get("error"))
                            .and_then(|sounds| sounds.first())
                            .or_else(|| {
                                interface.units.values().find_map(|unit| {
                                    unit.responses
                                        .get("error")
                                        .and_then(|sounds| sounds.first())
                                })
                            });
                        if let Some(sound) = sound {
                            frame.audio_started.push(StartedSound {
                                id: qualify_sound(
                                    &acquired.0.presentation.interface_package,
                                    sound,
                                ),
                                category: "effects".into(),
                                priority: 0,
                            });
                        }
                    }
                }
            }
        }
    }
    notices.0.extend(frame.notices.iter().cloned());
    while notices.0.len() > 32 {
        notices.0.remove(0);
    }
    if let Some(settings) = settings {
        audio_feed
            .categories
            .insert("master".into(), settings.master);
        audio_feed.categories.insert("voice".into(), settings.voice);
        audio_feed
            .categories
            .insert("alerts".into(), settings.alerts);
        audio_feed
            .categories
            .insert("effects".into(), settings.effects);
    }
}

fn push_notice(text: String, frame: &mut FrameFeedback) {
    let value = serde_json::json!({"text":text,"message":text});
    frame.notices.push(value);
}

fn display_content_name(id: &str) -> String {
    id.rsplit(':')
        .next()
        .unwrap_or(id)
        .split('_')
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .join(" ")
}

fn qualify_sound(package: &Option<String>, sound: &str) -> String {
    if sound.contains(':') {
        sound.to_string()
    } else {
        format!("{}:{sound}", package.as_deref().unwrap_or(""))
    }
}

#[derive(Clone)]
struct AlertEvent {
    rule_id: String,
    text: String,
    sound: Option<String>,
    minimap_ping: bool,
    unit: u32,
    position: Option<turts_net_protocol::Pos>,
    priority: i16,
}

fn collect_alert_events(
    current: &Replica,
    previous: Option<&Replica>,
    slot: u8,
    content: &crate::content::ClientContent,
    rules: &[turts_mod_api::AlertDef],
    now: f64,
    runtime: &mut FeedbackRuntime,
) -> Vec<AlertEvent> {
    let mut events = Vec::new();
    for rule in rules {
        let mut units = Vec::new();
        match &rule.trigger {
            AlertTrigger::OwnUnitDamaged => {
                if let Some(previous) = previous {
                    for health in &current.healths {
                        let owned = current
                            .units
                            .get(&health.unit.0)
                            .is_some_and(|unit| unit.owner_slot == slot);
                        let before = previous
                            .healths
                            .iter()
                            .find(|record| record.unit == health.unit)
                            .map(|record| record.current);
                        if owned && before.is_some_and(|before| health.current < before) {
                            units.push(health.unit.0);
                        }
                    }
                }
            }
            AlertTrigger::OwnUnitDestroyed => {
                if let Some(previous) = previous {
                    for (&id, unit) in &previous.units {
                        let destroyed = unit.owner_slot == slot
                            && !current.units.contains_key(&id)
                            && current
                                .health_trace
                                .get(&id)
                                .and_then(|trace| trace.last())
                                .is_some_and(|health| health.current == 0);
                        if destroyed {
                            units.push(id);
                        }
                    }
                }
            }
            AlertTrigger::ResearchComplete => {
                for research in &current.research {
                    if research.slot.0 == slot {
                        for completed in &research.completed {
                            let was_completed = previous
                                .and_then(|old| {
                                    old.research
                                        .iter()
                                        .find(|record| record.slot == research.slot)
                                })
                                .is_some_and(|old| old.completed.contains(completed));
                            if !was_completed
                                && let Some(previous) = previous
                                && let Some(id) = research_completion_producer(
                                    current, previous, slot, completed, content,
                                )
                            {
                                units.push(id);
                            }
                        }
                    }
                }
            }
            AlertTrigger::ExtractorFull => {
                for record in &current.extractors {
                    if record.status == ExtractorStatus::Full
                        && previous.is_some_and(|old| {
                            old.extractors
                                .iter()
                                .find(|r| r.unit == record.unit)
                                .is_some_and(|r| r.status != ExtractorStatus::Full)
                        })
                    {
                        units.push(record.unit.0);
                    }
                }
            }
            AlertTrigger::ExtractorDepleted => {
                for record in &current.extractors {
                    if record.status == ExtractorStatus::Depleted
                        && previous.is_some_and(|old| {
                            old.extractors
                                .iter()
                                .find(|r| r.unit == record.unit)
                                .is_some_and(|r| r.status != ExtractorStatus::Depleted)
                        })
                    {
                        units.push(record.unit.0);
                    }
                }
            }
            AlertTrigger::OutputBlocked => {
                for queue in &current.economy.queues {
                    if queue
                        .jobs
                        .iter()
                        .any(|job| job.stage == JobStage::OutputBlocked)
                        && previous.is_some_and(|old| {
                            old.economy
                                .queues
                                .iter()
                                .find(|q| q.producer == queue.producer)
                                .is_some_and(|q| {
                                    !q.jobs
                                        .iter()
                                        .any(|job| job.stage == JobStage::OutputBlocked)
                                })
                        })
                    {
                        units.push(queue.producer.0);
                    }
                }
            }
            AlertTrigger::OutOfAmmo => {
                for record in &current.weapons {
                    if record.status == WeaponStatus::OutOfAmmo
                        && previous.is_some_and(|old| {
                            old.weapons
                                .iter()
                                .find(|r| r.unit == record.unit)
                                .is_some_and(|r| r.status != WeaponStatus::OutOfAmmo)
                        })
                    {
                        units.push(record.unit.0);
                    }
                }
            }
            AlertTrigger::InserterWaiting(seconds) => {
                let mut waiting_units = BTreeSet::new();
                let tick = current.last_applied_tick.unwrap_or_default();
                for record in &current.inserters {
                    let waiting = matches!(
                        record.stage,
                        InserterStage::AtSource(Some(_)) | InserterStage::AtDestination(_)
                    );
                    let key = (rule.id.clone(), record.unit.0);
                    if waiting
                        && current
                            .units
                            .get(&record.unit.0)
                            .is_some_and(|unit| unit.owner_slot == slot)
                    {
                        waiting_units.insert(record.unit.0);
                        let started = *runtime.wait_started.entry(key).or_insert(tick);
                        let waited_ticks = tick.saturating_sub(started);
                        if waited_ticks as f64 / current.tick_hz.max(1) as f64 > *seconds as f64 {
                            units.push(record.unit.0);
                        }
                    } else {
                        runtime.wait_started.remove(&key);
                    }
                }
                runtime.wait_started.retain(|(rule_id, unit), _| {
                    rule_id != &rule.id || waiting_units.contains(unit)
                });
            }
            AlertTrigger::ResearchReady => {
                for (&id, unit) in &current.units {
                    if unit.owner_slot == slot
                        && research_can_start(current, id, slot, content)
                        && previous.is_none_or(|old| !research_can_start(old, id, slot, content))
                    {
                        units.push(id);
                    }
                }
            }
            AlertTrigger::ProductionComplete => {
                if let Some(previous) = previous {
                    for old_queue in &previous.economy.queues {
                        for job in old_queue.jobs.iter().filter(|job| {
                            matches!(job.stage, JobStage::Running | JobStage::OutputBlocked)
                        }) {
                            let still_running = current
                                .economy
                                .queues
                                .iter()
                                .find(|queue| queue.producer == old_queue.producer)
                                .is_some_and(|queue| queue.jobs.iter().any(|now| now.id == job.id));
                            let output_produced = output_evidence(
                                previous,
                                current,
                                old_queue.producer,
                                &job.recipe,
                                content,
                            );
                            if !still_running
                                && output_produced
                                && current
                                    .units
                                    .get(&old_queue.producer.0)
                                    .is_some_and(|unit| unit.owner_slot == slot)
                            {
                                units.push(old_queue.producer.0);
                            }
                        }
                    }
                }
            }
            AlertTrigger::GathererWaiting(seconds) => {
                let tick = current.last_applied_tick.unwrap_or_default();
                let mut waiting_units = BTreeSet::new();
                for order in &current.economy.orders {
                    let owned = current
                        .units
                        .get(&order.unit.0)
                        .is_some_and(|unit| unit.owner_slot == slot);
                    let key = (rule.id.clone(), order.unit.0);
                    if owned && matches!(order.stage, GatherStage::Waiting { .. }) {
                        waiting_units.insert(order.unit.0);
                        let started = *runtime.wait_started.entry(key).or_insert(tick);
                        let waited_ticks = tick.saturating_sub(started);
                        if waited_ticks as f64 / current.tick_hz.max(1) as f64 > *seconds as f64 {
                            units.push(order.unit.0);
                        }
                    } else {
                        runtime.wait_started.remove(&key);
                    }
                }
                runtime.wait_started.retain(|(rule_id, unit), _| {
                    rule_id != &rule.id || waiting_units.contains(unit)
                });
            }
        }
        if !matches!(&rule.trigger, AlertTrigger::OwnUnitDestroyed) {
            units.retain(|unit| {
                current
                    .units
                    .get(unit)
                    .is_some_and(|record| record.owner_slot == slot)
            });
        }
        if units.is_empty() {
            continue;
        }
        let last = runtime.cooldowns.get(&rule.id).copied();
        if last.is_some_and(|last| now - last < rule.cooldown.max(1.0) as f64) {
            continue;
        }
        runtime.cooldowns.insert(rule.id.clone(), now);
        // A wait is a state: it alerts once when it passes its threshold, not again while it
        // lasts. A new wait starts when the unit stops waiting and waits again.
        if matches!(
            rule.trigger,
            AlertTrigger::InserterWaiting(_) | AlertTrigger::GathererWaiting(_)
        ) {
            for unit in &units {
                if let Some(started) = runtime.wait_started.get_mut(&(rule.id.clone(), *unit)) {
                    *started = WAIT_ALERTED;
                }
            }
        }
        let mut seen = BTreeSet::new();
        for unit in units {
            if unit != 0 && !seen.insert(unit) {
                continue;
            }
            let position = current
                .units
                .get(&unit)
                .or_else(|| previous.and_then(|old| old.units.get(&unit)))
                .and_then(|record| record.samples.back().map(|(_, pos)| *pos));
            if position.is_some_and(|position| {
                events.iter().any(|event: &AlertEvent| {
                    event.rule_id == rule.id
                        && event.position.is_some_and(|prior| {
                            distance_squared(prior, position)
                                <= rule.coalesce_radius.max(0.0).powi(2)
                        })
                })
            }) {
                continue;
            }
            events.push(AlertEvent {
                rule_id: rule.id.clone(),
                text: rule.text.clone(),
                sound: rule.sound.clone(),
                minimap_ping: rule.minimap_ping,
                unit,
                position,
                priority: rule.priority,
            });
        }
    }
    events.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    events
}

fn research_can_start(
    replica: &Replica,
    station_id: u32,
    slot: u8,
    content: &crate::content::ClientContent,
) -> bool {
    let Some(unit) = replica
        .units
        .get(&station_id)
        .filter(|unit| unit.owner_slot == slot)
    else {
        return false;
    };
    let Ok(unit_id) = turts_mod_api::ContentId::parse(&unit.content_id) else {
        return false;
    };
    let Some(station) = content
        .catalog
        .units
        .get(&unit_id)
        .and_then(|unit| unit.research.as_ref())
    else {
        return false;
    };
    let completed = replica
        .research
        .iter()
        .find(|record| record.slot.0 == slot)
        .map(|record| &record.completed);
    let queue = replica
        .economy
        .queues
        .iter()
        .find(|queue| queue.producer.0 == station_id);
    let queue_enabled = queue.is_none_or(|queue| queue.enabled);
    let queue_len = queue.map_or(0, |queue| queue.jobs.len());
    let queue_has_room = queue_len < usize::from(station.queue_capacity);
    station.technologies.iter().any(|technology_id| {
        let done = completed.is_some_and(|items| items.contains(&technology_id.to_string()));
        let Some(technology) = content.catalog.technologies.get(technology_id) else {
            return false;
        };
        let prerequisites_met = technology.prerequisites.iter().all(|prerequisite| {
            completed.is_some_and(|items| items.contains(&prerequisite.to_string()))
        });
        let in_progress = replica.economy.queues.iter().any(|queue| {
            replica
                .units
                .get(&queue.producer.0)
                .is_some_and(|producer| producer.owner_slot == slot)
                && queue
                    .jobs
                    .iter()
                    .any(|job| job.recipe == technology.id.to_string())
        });
        let costs_met = research_costs_affordable(replica, station_id, &technology.costs);
        !done && !in_progress && prerequisites_met && queue_enabled && queue_has_room && costs_met
    })
}

fn research_completion_producer(
    current: &Replica,
    previous: &Replica,
    slot: u8,
    technology: &str,
    content: &crate::content::ClientContent,
) -> Option<u32> {
    // A completed technology is often removed from the queue in the same
    // snapshot that adds it to ResearchRecord, so inspect both sides of the
    // transition. The queue's producer identifies the station whose location
    // should receive the completion ping.
    for replica in [previous, current] {
        for queue in &replica.economy.queues {
            if !queue.jobs.iter().any(|job| job.recipe == technology) {
                continue;
            }
            let producer = queue.producer.0;
            let owner = current
                .units
                .get(&producer)
                .or_else(|| previous.units.get(&producer));
            if owner.is_some_and(|unit| unit.owner_slot == slot)
                && current
                    .units
                    .get(&producer)
                    .or_else(|| previous.units.get(&producer))
                    .is_some_and(|unit| {
                        turts_mod_api::ContentId::parse(&unit.content_id)
                            .ok()
                            .and_then(|id| content.catalog.units.get(&id))
                            .is_some_and(|definition| definition.research.is_some())
                    })
            {
                return Some(producer);
            }
        }
    }

    // Some servers may publish the completion record without retaining its
    // originating job. Fall back only to an actually owned research station.
    current.units.iter().find_map(|(&id, unit)| {
        if unit.owner_slot != slot {
            return None;
        }
        let content_id = turts_mod_api::ContentId::parse(&unit.content_id).ok()?;
        content
            .catalog
            .units
            .get(&content_id)
            .filter(|definition| definition.research.is_some())
            .map(|_| id)
    })
}

fn research_costs_affordable(
    replica: &Replica,
    station_id: u32,
    costs: &[turts_mod_api::ResolvedCost],
) -> bool {
    use turts_mod_api::CostSource;
    let mut required = BTreeMap::<(CostSource, turts_mod_api::ContentId), u128>::new();
    for cost in costs {
        let amount = required
            .entry((cost.source, cost.resource.clone()))
            .or_default();
        let Some(sum) = amount.checked_add(u128::from(cost.amount)) else {
            return false;
        };
        *amount = sum;
    }
    required.into_iter().all(|((source, resource), amount)| {
        let Ok(amount) = u64::try_from(amount) else {
            return false;
        };
        let resource = resource.to_string();
        let available = match source {
            CostSource::SlotBalance => replica
                .economy
                .balances
                .iter()
                .find(|balance| balance.resource == resource)
                .and_then(|balance| balance.on_hand.checked_sub(balance.reserved)),
            CostSource::ProducerInventory => replica
                .economy
                .inventories
                .iter()
                .find(|inventory| {
                    inventory.unit.0 == station_id
                        && inventory.port == turts_net_protocol::InventoryPort::Main
                })
                .and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .find(|item| item.resource == resource)
                })
                .and_then(|balance| balance.on_hand.checked_sub(balance.reserved)),
        };
        amount == 0 || available.is_some_and(|available| available >= amount)
    })
}

fn inventory_amount(
    replica: &Replica,
    unit: turts_net_protocol::UnitId,
    item: &str,
    output_only: bool,
) -> u64 {
    replica
        .economy
        .inventories
        .iter()
        .filter(|inventory| {
            inventory.unit == unit
                && (!output_only || inventory.port == turts_net_protocol::InventoryPort::Output)
        })
        .flat_map(|inventory| inventory.items.iter())
        .filter(|balance| balance.resource == item)
        .map(|balance| balance.on_hand)
        .sum()
}

fn output_evidence(
    previous: &Replica,
    current: &Replica,
    producer: turts_net_protocol::UnitId,
    recipe: &str,
    content: &crate::content::ClientContent,
) -> bool {
    let Ok(recipe_id) = turts_mod_api::ContentId::parse(recipe) else {
        return false;
    };
    if let Some(production) = content.catalog.production.get(&recipe_id) {
        let output_id = production.output_unit.to_string();
        let Some(owner_slot) = previous.units.get(&producer.0).map(|unit| unit.owner_slot) else {
            return false;
        };
        let before = previous
            .units
            .values()
            .filter(|unit| unit.owner_slot == owner_slot && unit.content_id == output_id)
            .count();
        let after = current
            .units
            .values()
            .filter(|unit| unit.owner_slot == owner_slot && unit.content_id == output_id)
            .count();
        if after > before {
            return true;
        }
    }
    content
        .catalog
        .recipes
        .get(&recipe_id)
        .is_some_and(|recipe| {
            !recipe.outputs.is_empty()
                && recipe.outputs.iter().all(|output| {
                    let before =
                        inventory_amount(previous, producer, &output.item.to_string(), true);
                    let after = inventory_amount(current, producer, &output.item.to_string(), true);
                    after.saturating_sub(before) >= output.amount
                })
        })
}

fn distance_squared(a: turts_net_protocol::Pos, b: turts_net_protocol::Pos) -> f32 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2)
}
