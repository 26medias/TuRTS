//! Public profile host. All effects pass through the common actor admission path.
mod observation;
mod projection;

use crate::{
    actors,
    config::{BotConfig, BotLimitsConfig},
    orders::ServerCatalog,
    permissions::AllVisible,
    session,
};
use bevy::prelude::*;
use observation::{CandidateLease, ObservationVerifier};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use turts_bot::*;
use turts_mod_api::{BotDef, BotQuota, BotStartWhen, ContentId, WorldPos};
use turts_sim::{MatchConfig, SimClock, SimSet, Tick};

pub(crate) fn operator_quota() -> BotQuota {
    BotQuota {
        queries: 32,
        candidates: 4096,
        rows: 1024,
        proposals: 32,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum BotExecutionMode {
    Live,
    RecordedCommands,
    RegenerateAndCompare,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LifecycleInput {
    pub scheduler_tick: u32,
    pub human_present: bool,
    pub running: bool,
    pub ended: bool,
    pub defeated: Vec<u8>,
}
/// Replay installs this resource before advancing a tick. No live presence is read in replay.
#[derive(Resource)]
pub(crate) struct BotExecution {
    pub mode: BotExecutionMode,
    pub inputs: BTreeMap<u32, LifecycleInput>,
    pub generated: Vec<EmittedCommand>,
    pub input_trace: Vec<LifecycleInput>,
    pub error: Option<String>,
    last_scheduler_tick: Option<u32>,
}
impl Default for BotExecution {
    fn default() -> Self {
        Self {
            mode: BotExecutionMode::Live,
            inputs: BTreeMap::new(),
            generated: Vec::new(),
            input_trace: Vec::new(),
            error: None,
            last_scheduler_tick: None,
        }
    }
}
struct BotInstance {
    actor: BotActor,
    profile: BotDef,
    controllers: Vec<ControllerInstance>,
    coordinator: BotCoordinator,
    period: u64,
    clock: ActiveClock,
    decision_step: u64,
    next_basis: u64,
    lease: Option<CandidateLease>,
    last_diagnostic: Option<String>,
    last_failure_step: Option<u64>,
}
#[derive(Default)]
struct ActiveClock {
    last_tick: Option<u32>,
    active: u64,
    next_due: u64,
    paused: bool,
}
impl ActiveClock {
    fn advance(&mut self, tick: u32, eligible: bool, period: u64) -> bool {
        if self.last_tick == Some(tick) {
            return false;
        }
        self.last_tick = Some(tick);
        if !eligible {
            self.paused = true;
            return false;
        }
        self.paused = false;
        let Some(active) = self.active.checked_add(1) else {
            return false;
        };
        self.active = active;
        if self.active < self.next_due {
            return false;
        }
        self.next_due = self.active.saturating_add(period);
        true
    }
}
#[derive(Resource)]
pub(crate) struct BotHost {
    bots: Vec<BotInstance>,
    catalog: Arc<turts_mod_api::ContentCatalog>,
    pub private_projection_scans: u64,
    pub decisions: u64,
    limits: BotLimitsConfig,
    installation: turts_audit::ActorInstallation,
    installation_recorded: bool,
}
impl BotHost {
    pub fn actors(&self) -> Vec<BotActor> {
        self.bots.iter().map(|b| b.actor.clone()).collect()
    }
}
/// Validate every profile and factory before the atomic actor claim batch.
pub(crate) fn install(app: &mut App, configuration: &[BotConfig]) -> Result<(), String> {
    let mut registry = ControllerRegistry::new();
    turts_bot::controllers::register_stock(&mut registry).map_err(|e| e.to_string())?;
    install_with_registry_and_limits(app, configuration, &registry, BotLimitsConfig::default())
}
pub(crate) fn install_with_limits(
    app: &mut App,
    configuration: &[BotConfig],
    limits: BotLimitsConfig,
) -> Result<(), String> {
    let mut registry = ControllerRegistry::new();
    turts_bot::controllers::register_stock(&mut registry).map_err(|e| e.to_string())?;
    install_with_registry_and_limits(app, configuration, &registry, limits)
}
pub(crate) fn install_with_registry(
    app: &mut App,
    configuration: &[BotConfig],
    registry: &ControllerRegistry,
) -> Result<(), String> {
    install_with_registry_and_limits(app, configuration, registry, BotLimitsConfig::default())
}
fn install_with_registry_and_limits(
    app: &mut App,
    configuration: &[BotConfig],
    registry: &ControllerRegistry,
    limits: BotLimitsConfig,
) -> Result<(), String> {
    if configuration.is_empty() {
        return Ok(());
    }
    if app.world().contains_resource::<BotHost>() {
        return Err("bots are already installed for this match".into());
    }
    let catalog = Arc::new(app.world().resource::<ServerCatalog>().0.clone());
    let hz = app.world().resource::<SimClock>().tick_hz;
    let mut slots = BTreeSet::new();
    let mut bots = Vec::new();
    let mut installed_bots = Vec::new();
    for (index, config) in configuration.iter().enumerate() {
        let field = format!("bots[{index}]");
        if !slots.insert(config.slot) {
            return Err(format!("{field}.slot: duplicate bot slot"));
        }
        let id = ContentId::parse(&config.profile).map_err(|e| format!("{field}.profile: {e}"))?;
        let profile = catalog
            .bots
            .get(&id)
            .ok_or_else(|| format!("{field}.profile: unknown profile `{id}`"))?
            .clone();
        let period = u64::from(profile.decision_ms)
            .checked_mul(u64::from(hz))
            .and_then(|ticks| ticks.checked_add(999))
            .map(|ticks| ticks / 1000)
            .ok_or_else(|| format!("{field}.decision_ms: decision clock overflow"))?;
        if profile.decision_ms == 0 || period == 0 {
            return Err(format!(
                "{field}.decision_ms: decision period must be positive"
            ));
        }
        let actor = BotActor {
            slot: config.slot,
            profile: id,
        };
        installed_bots.push(turts_audit::ActorBotInstallation {
            slot: config.slot,
            profile: actor.profile.clone(),
            seed: config.seed,
            profile_hash: turts_audit::actor_profile_hash(&profile)
                .map_err(|e| format!("{field}.profile: {e}"))?,
        });
        let controllers = registry
            .activate(
                &actor,
                &profile,
                config.seed,
                BotQuota {
                    queries: limits.queries,
                    candidates: limits.candidates,
                    rows: limits.rows,
                    proposals: limits.proposals,
                },
            )
            .map_err(|e| format!("{field}.{e}"))?;
        bots.push(BotInstance {
            coordinator: BotCoordinator::new(actor.clone(), coordinator_limits(limits)),
            actor,
            profile,
            controllers,
            period,
            clock: ActiveClock::default(),
            decision_step: 0,
            next_basis: 0,
            lease: None,
            last_diagnostic: None,
            last_failure_step: None,
        });
    }
    let installation = turts_audit::ActorInstallation {
        locked_package_hash: app
            .world()
            .get_resource::<session::Advertised>()
            .ok_or("bot installation requires a locked package advertisement")?
            .lockfile_hash
            .clone(),
        engine_api: turts_mod_api::ENGINE_API_VERSION.to_string(),
        protocol: u32::from(turts_net_protocol::NET_PROTOCOL_VERSION),
        bots: installed_bots,
        limits: artifact_limits(limits)?,
    };
    actors::install_bot_actors(
        app.world_mut(),
        &bots.iter().map(|b| b.actor.clone()).collect::<Vec<_>>(),
    )?;
    app.init_resource::<BotExecution>();
    app.insert_resource(BotHost {
        bots,
        catalog,
        private_projection_scans: 0,
        decisions: 0,
        limits,
        installation,
        installation_recorded: false,
    });
    app.add_systems(
        FixedPreUpdate,
        drive
            .after(SimSet::Construction)
            .before(session::admit_player_commands)
            .before(SimSet::Commands),
    );
    Ok(())
}
fn artifact_limits(limits: BotLimitsConfig) -> Result<turts_audit::ActorOperatorLimits, String> {
    Ok(turts_audit::ActorOperatorLimits {
        queries: limits.queries,
        candidates: limits.candidates,
        rows: limits.rows,
        proposals: limits.proposals,
        commands: limits.commands,
        affected_units: limits.affected_units,
        pending_requests: limits.pending_requests,
        pending_accounts: limits.pending_accounts,
        projection_rows: u64::try_from(limits.projection_rows)
            .map_err(|_| "bot projection limit does not fit the replay artifact")?,
        lease_steps: 8,
        page_rows: 64,
        cursor_records: 1024,
    })
}

fn coordinator_limits(limits: BotLimitsConfig) -> CoordinatorLimits {
    CoordinatorLimits {
        commands: limits.commands,
        affected_units: limits.affected_units,
        pending_requests: limits.pending_requests,
        pending_accounts: limits.pending_accounts,
    }
}

fn diagnose(bot: &mut BotInstance, message: String) {
    bot.last_failure_step = Some(bot.decision_step);
    if bot.last_diagnostic.as_ref() != Some(&message) {
        eprintln!("turts_server: bot slot {}: {message}", bot.actor.slot);
        bot.last_diagnostic = Some(message);
    }
}
fn lifecycle(world: &World, execution: &BotExecution, tick: u32) -> Result<LifecycleInput, String> {
    if execution.mode != BotExecutionMode::Live {
        return execution
            .inputs
            .get(&tick)
            .cloned()
            .ok_or_else(|| format!("bot replay lacks lifecycle input for tick {tick}"));
    }
    let outcomes = world.get_resource::<turts_sim::outcome::MatchOutcomes>();
    Ok(LifecycleInput {
        scheduler_tick: tick,
        human_present: actors::human_present(world),
        running: world.contains_resource::<MatchConfig>(),
        ended: outcomes.is_some_and(|o| o.end_tick.is_some()),
        defeated: outcomes
            .into_iter()
            .flat_map(|o| o.slots.iter().enumerate())
            .filter_map(|(slot, result)| {
                result
                    .filter(|r| r.outcome == turts_sim::outcome::Outcome::Defeat)
                    .map(|_| slot as u8)
            })
            .collect(),
    })
}
pub(crate) fn drive(world: &mut World) {
    let Some(mut host) = world.remove_resource::<BotHost>() else {
        return;
    };
    let mut execution = world.remove_resource::<BotExecution>().unwrap_or_default();
    drive_inner(world, &mut host, &mut execution);
    world.insert_resource(execution);
    world.insert_resource(host);
}
fn drive_inner(world: &mut World, host: &mut BotHost, execution: &mut BotExecution) {
    // Recorded-command replay never regenerates submissions, including its first tick.
    if execution.mode == BotExecutionMode::RecordedCommands {
        return;
    }
    if !host.installation_recorded {
        if let Err(error) = actors::record_installation(
            world,
            turts_audit::ActorLogRecord::Installation(host.installation.clone()),
        ) {
            execution.error = Some(error);
            return;
        }
        host.installation_recorded = true;
    }
    let tick = world.resource::<Tick>().0;
    if execution.last_scheduler_tick == Some(tick) {
        return;
    }
    execution.last_scheduler_tick = Some(tick);
    // These are one-tick publication buffers. The trusted log consumes them every completed tick.
    // They never retain an unbounded match history inside a live bot instance.
    execution.generated.clear();
    execution.input_trace.clear();
    let input = match lifecycle(world, execution, tick) {
        Ok(input) => input,
        Err(error) => {
            execution.error = Some(error);
            return;
        }
    };
    let phase = if input.ended {
        turts_audit::ActorPhase::Ended
    } else if input.running {
        turts_audit::ActorPhase::Running
    } else {
        turts_audit::ActorPhase::Waiting
    };
    if let Err(error) = actors::record_lifecycle(
        world,
        turts_audit::ActorLogRecord::Lifecycle {
            tick,
            humans_present: input.human_present,
            phase,
        },
    ) {
        execution.error = Some(error);
        return;
    }
    if execution
        .input_trace
        .last()
        .is_none_or(|old| old.scheduler_tick != tick)
    {
        execution.input_trace.push(input.clone())
    }
    let hz = world.resource::<SimClock>().tick_hz;
    let teams = world
        .get_resource::<MatchConfig>()
        .map_or_else(Vec::new, |c| c.teams.clone());
    for bot in &mut host.bots {
        let eligible = input.running
            && !input.ended
            && !input.defeated.contains(&bot.actor.slot)
            && (bot.profile.start_when == BotStartWhen::MatchRunning || input.human_present);
        if !eligible {
            bot.lease = None;
        }
        if !bot.clock.advance(tick, eligible, bot.period) {
            continue;
        }
        bot.decision_step = bot.decision_step.saturating_add(1);
        host.decisions = host.decisions.saturating_add(1);
        for result in actors::actor_results(world, &bot.actor) {
            if let Err(error) = bot.coordinator.observe_result(result) {
                diagnose(bot, format!("result reconciliation: {error:?}"))
            }
        }
        let batch = match actors::actor_reconciliation(world, &bot.actor) {
            Ok(batch) => batch,
            Err(error) => {
                diagnose(bot, error);
                continue;
            }
        };
        let Some(next_basis) = bot
            .next_basis
            .checked_add(1)
            .filter(|value| *value < (1u64 << 56))
        else {
            diagnose(bot, "observation sequence exhausted".into());
            continue;
        };
        bot.next_basis = next_basis;
        // A different bot's private accounting or scheduler outcome cannot change these IDs.
        let accounting_basis = (u64::from(bot.actor.slot) << 56) | next_basis;
        if let Err(error) = bot.coordinator.reconcile(accounting_basis, &batch) {
            diagnose(bot, format!("accounting basis refused: {error:?}"));
            continue;
        }
        let view = turts_sim::completed_world(world);
        if projection::source_rows(view) > host.limits.projection_rows {
            diagnose(bot, "observation host row limit reached".into());
            continue;
        }
        let size = world.resource::<turts_sim::MapBounds>().size;
        let projected = projection::project(
            view,
            bot.actor.slot,
            accounting_basis,
            host.catalog.clone(),
            teams.clone(),
            WorldPos {
                x: size.x,
                y: size.y,
            },
            &AllVisible,
        );
        host.private_projection_scans = host
            .private_projection_scans
            .saturating_add(projected.private_scanned as u64);
        let current = projected.basis;
        // Release/expiry is applied between decisions, never before proposal verification.
        if bot
            .lease
            .as_ref()
            .is_none_or(|lease| lease.expired(bot.decision_step) || !lease.retained())
        {
            // A candidate basis owns only permitted immutable facts. Account batches remain fresh.
            bot.lease = Some(CandidateLease::new(current.clone(), bot.decision_step));
        }
        let counts = role_counts(&current, &bot.profile);
        let supply = view
            .operating
            .as_ref()
            .into_iter()
            .flat_map(|op| op.supply.iter())
            .filter(|s| s.slot == bot.actor.slot)
            .map(|s| SupplyObservation {
                resource: s.resource.clone(),
                provided: s.provided,
                consumed: s.consumed,
            })
            .collect::<Vec<_>>();
        let Some(lease) = bot.lease.as_mut() else {
            diagnose(bot, "candidate observation basis is missing".into());
            continue;
        };
        let mut proposals = Vec::new();
        for controller in &mut bot.controllers {
            let frame = BotFrame {
                basis: lease.basis().id,
                accounting_basis,
                completed_tick: view.tick,
                scheduler_tick: tick,
                active_ticks: bot.clock.active,
                actor: &bot.actor,
                tick_hz: hz,
                phase: MatchPhase::Running,
                profile: &bot.profile,
                quota: controller.quota,
                limits: coordinator_limits(host.limits),
                reconciliation: &batch,
                role_counts: &counts,
                supply: &supply,
            };
            let mut buffer = ProposalBuffer::new(
                &controller.key,
                controller.quota.proposals,
                host.limits.affected_units,
            );
            controller.controller.step(
                &frame,
                &mut lease.queries(&controller.key, controller.quota),
                &mut buffer,
            );
            proposals.extend(buffer.into_proposals());
        }
        let selection = bot.coordinator.select(
            accounting_basis,
            proposals,
            &ObservationVerifier {
                candidate: lease,
                current: &current,
                profile: &bot.profile,
            },
        );
        for command in selection.commands {
            execution.generated.push(command.clone());
            if let Err(error) = actors::stage_bot_command(world, command) {
                diagnose(bot, format!("command admission refused: {error:?}"))
            }
        }
        if let Err(error) = actors::acknowledge(
            world,
            &bot.actor,
            batch.watermark,
            &bot.coordinator.tracked_requests(),
        ) {
            diagnose(bot, error)
        }
        if bot.last_failure_step != Some(bot.decision_step) {
            bot.last_diagnostic = None;
        }
    }
}
fn role_counts(basis: &observation::ObservationBasis, profile: &BotDef) -> Vec<RoleCount> {
    profile
        .knowledge
        .roles
        .iter()
        .map(|role| {
            let accepts = |id: &ContentId| role.units.iter().any(|s| s == &id.to_string());
            RoleCount {
                role: role.key.clone(),
                completed: basis
                    .units
                    .iter()
                    .filter(|u| {
                        u.owner_slot == basis.slot
                            && !u.site
                            && accepts(&u.content)
                            && role.requires.iter().all(|c| u.capabilities.contains(c))
                    })
                    .count() as u32,
                pending: basis
                    .works
                    .iter()
                    .filter(|w| !w.state.terminal() && w.content.as_ref().is_some_and(accepts))
                    .count() as u32
                    + basis
                        .pending_content
                        .iter()
                        .filter(|(content, _)| accepts(content))
                        .map(|(_, count)| *count as u32)
                        .sum::<u32>(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_uses_active_time_and_resumes_without_catch_up() {
        let mut clock = ActiveClock::default();
        assert!(clock.advance(0, true, 3));
        assert!(!clock.advance(1, true, 3));
        for tick in 2..100 {
            assert!(!clock.advance(tick, false, 3));
        }
        assert!(!clock.advance(100, true, 3));
        assert!(clock.advance(101, true, 3));
        assert_eq!(clock.active, 4);
        assert!(!clock.advance(101, true, 3));
        assert!(!clock.advance(1000, true, 3));
        assert_eq!(
            clock.active, 5,
            "scheduler gaps never become active-time catch-up"
        );
    }
}

#[cfg(test)]
mod scenarios;

#[cfg(test)]
mod replay;
