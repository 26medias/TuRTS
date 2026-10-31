//! Protocol 4 order resolution for the server's package catalog.

use bevy::prelude::Resource;
use turts_mod_api::{Behaviour, ContentCatalog, ContentId};
use turts_net_protocol::{
    CommandRejectReason, MAX_ORDER_UNITS, OrderResult, OrderTarget, PlayerCommand, UnitId,
    UnitRefusal,
};
use turts_sim::{
    Command, CommandKind, RejectReason, SlotId,
    orders::{OrderCommand, OrderTargetSim, QueueMode},
};

#[derive(Resource, Clone)]
pub struct ServerCatalog(pub ContentCatalog);

/// Resolve a mod-owned order through its data definition and adapt supported M2/M3 behaviours.
pub fn to_sim(
    slot: SlotId,
    command: &PlayerCommand,
    catalog: &ContentCatalog,
) -> Result<Vec<Command>, OrderResult> {
    let order = &command.order;
    // K4 limits every envelope, including behaviours that expand to one sim command per unit.
    if order.units.len() > MAX_ORDER_UNITS {
        return Err(refusal(command, CommandRejectReason::TooManyUnits));
    }
    let Ok(order_id) = ContentId::parse(&order.order) else {
        return Err(refusal(command, CommandRejectReason::UnknownOrder));
    };
    let Some(definition) = catalog.orders.get(&order_id) else {
        return Err(refusal(command, CommandRejectReason::UnknownOrder));
    };
    if order.units.is_empty()
        && !matches!(
            (definition.behaviour, &order.target),
            (Behaviour::Move, OrderTarget::Point(_))
        )
    {
        return Err(refusal(command, CommandRejectReason::EmptyCommand));
    }
    let target = match &order.target {
        OrderTarget::None => OrderTargetSim::None,
        OrderTarget::Point(point) => OrderTargetSim::Point(bevy::math::Vec2::new(point.x, point.y)),
        OrderTarget::Unit(unit) => OrderTargetSim::Unit(unit.0),
        OrderTarget::Node(node) => OrderTargetSim::Node(turts_sim::NodeId(node.0)),
        OrderTarget::Job(job) => OrderTargetSim::Job(turts_sim::JobId(job.0)),
        OrderTarget::Content(content) => ContentId::parse(content)
            .map(OrderTargetSim::Content)
            .unwrap_or(OrderTargetSim::Invalid),
    };
    let order_package = order_id.package.clone();
    let queue_depth = catalog
        .order_queue_depths
        .get(&order_package)
        .copied()
        .unwrap_or(turts_mod_api::DEFAULT_QUEUE_DEPTH);
    let sim_order = OrderCommand {
        order: order_id.clone(),
        behaviour: definition.behaviour,
        target_kind: definition.target,
        policy: Some(turts_sim::OrderPolicyAudit {
            id: order_id.clone(),
            behaviour: definition.behaviour,
            target: definition.target,
            units: definition.units.clone(),
            queue_depth,
            unit_capabilities: catalog
                .units
                .iter()
                .map(|(id, unit)| turts_sim::OrderUnitCapabilitiesAudit {
                    unit: id.to_string(),
                    movement: unit.movement.is_some(),
                    weapon: unit.weapon.is_some(),
                    gatherer: unit.gatherer.is_some(),
                    producer: unit.producer.is_some(),
                    research: unit.research.is_some(),
                    removable: unit
                        .removal
                        .as_ref()
                        .is_some_and(|policy| policy.by_command),
                    toggleable: unit.extractor.is_some()
                        || unit.inserter.is_some()
                        || unit.machine.is_some()
                        || unit.research.is_some()
                        || unit.producer.is_some()
                        || unit.weapon.is_some(),
                })
                .collect(),
        }),
        units: order.units.iter().map(|unit| unit.0).collect(),
        target,
        queue: match order.queue {
            turts_net_protocol::QueueMode::Replace => QueueMode::Replace,
            turts_net_protocol::QueueMode::Append => QueueMode::Append,
        },
        group: order.group,
        allowed_units: definition.units.clone(),
        queue_depth,
    };
    Ok(vec![Command {
        slot,
        seq: command.seq,
        kind: CommandKind::Order(Box::new(sim_order)),
    }])
}

/// Convert simulation per-unit outcomes to the protocol result.
pub fn to_wire(results: &[turts_sim::CommandResult], command: &PlayerCommand) -> OrderResult {
    let mut wire_result = OrderResult {
        seq: command.seq,
        tick: 0,
        accepted: Vec::new(),
        refused: Vec::new(),
        envelope: None,
        job_id: None,
    };
    if let Some(sim_result) = results.first()
        && (!sim_result.units.accepted.is_empty() || !sim_result.units.refused.is_empty())
    {
        wire_result.accepted = sim_result
            .units
            .accepted
            .iter()
            .copied()
            .map(UnitId)
            .collect();
        wire_result.refused = sim_result
            .units
            .refused
            .iter()
            .map(|(unit, reason)| UnitRefusal {
                unit: UnitId(*unit),
                reason: wire_reason(*reason),
            })
            .collect();
        wire_result.job_id = sim_result
            .outcome
            .as_ref()
            .ok()
            .and_then(|applied| applied.job_id)
            .map(|job| turts_net_protocol::JobId(job.0));
        if wire_result.accepted.is_empty() && command.order.units.is_empty() {
            wire_result.envelope = wire_result.refused.first().map(|refusal| refusal.reason);
        }
        return wire_result;
    }
    if let Some(outcome) = results.first().map(|result| &result.outcome) {
        match outcome {
            Ok(applied) => {
                wire_result.accepted.clone_from(&command.order.units);
                wire_result.job_id = applied.job_id.map(|job| turts_net_protocol::JobId(job.0));
            }
            Err(reason) => {
                let reason = wire_reason(*reason);
                if command.order.units.is_empty() {
                    wire_result.envelope = Some(reason);
                } else {
                    wire_result.refused = refusals(&command.order.units, reason);
                }
            }
        }
    } else if command.order.units.is_empty() {
        wire_result.envelope = Some(CommandRejectReason::Internal);
    } else {
        wire_result.refused = refusals(&command.order.units, CommandRejectReason::Internal);
    }
    wire_result
}

pub fn refusal(command: &PlayerCommand, reason: CommandRejectReason) -> OrderResult {
    OrderResult {
        seq: command.seq,
        tick: 0,
        accepted: Vec::new(),
        refused: refusals(&command.order.units, reason),
        envelope: command.order.units.is_empty().then_some(reason),
        job_id: None,
    }
}

fn refusals(units: &[UnitId], reason: CommandRejectReason) -> Vec<UnitRefusal> {
    units
        .iter()
        .copied()
        .map(|unit| UnitRefusal { unit, reason })
        .collect()
}

fn wire_reason(reason: RejectReason) -> CommandRejectReason {
    use RejectReason as R;
    match reason {
        R::EmptyCommand => CommandRejectReason::EmptyCommand,
        R::TooManyUnits => CommandRejectReason::TooManyUnits,
        R::DuplicateUnit => CommandRejectReason::DuplicateUnit,
        R::UnknownUnit => CommandRejectReason::UnknownUnit,
        R::NotOwner => CommandRejectReason::NotOwner,
        R::NotMobile => CommandRejectReason::NotMobile,
        R::TargetNotFinite => CommandRejectReason::TargetNotFinite,
        R::TargetOutsideMap => CommandRejectReason::TargetOutsideMap,
        R::MissingCapability => CommandRejectReason::MissingCapability,
        R::UnknownNode => CommandRejectReason::UnknownNode,
        R::IncompatibleItem => CommandRejectReason::IncompatibleItem,
        R::NodeDepleted => CommandRejectReason::NodeDepleted,
        R::UnknownRecipe => CommandRejectReason::UnknownRecipe,
        R::RecipeNotEnabled => CommandRejectReason::RecipeNotEnabled,
        R::QueueFull => CommandRejectReason::QueueFull,
        R::InsufficientBalance | R::UnknownBalance => CommandRejectReason::InsufficientBalance,
        R::InsufficientItems | R::UnknownInventory | R::InvalidAmount => {
            CommandRejectReason::InsufficientItems
        }
        R::UnknownJob => CommandRejectReason::UnknownJob,
        R::RefundDestinationFull => CommandRejectReason::RefundDestinationFull,
        R::RefundDestinationOverflow => CommandRejectReason::RefundDestinationOverflow,
        R::ArithmeticOverflow => CommandRejectReason::ArithmeticOverflow,
        R::IdExhausted => CommandRejectReason::IdExhausted,
        R::BadSequence => CommandRejectReason::BadSequence,
        R::NotRemovable => CommandRejectReason::NotRemovable,
        R::NotToggleable => CommandRejectReason::NotToggleable,
        R::PrerequisitesMissing => CommandRejectReason::PrerequisitesMissing,
        R::AlreadyResearched => CommandRejectReason::AlreadyResearched,
        R::ResearchInProgress => CommandRejectReason::ResearchInProgress,
        R::IntegrityViolation => CommandRejectReason::Internal,
        R::NotApplicable => CommandRejectReason::NotApplicable,
        R::TargetInvalid => CommandRejectReason::TargetInvalid,
        R::TargetGone => CommandRejectReason::TargetGone,
        R::OutOfRange => CommandRejectReason::OutOfRange,
        R::NotQueueable => CommandRejectReason::NotQueueable,
        R::Blocked => CommandRejectReason::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_net_protocol::OrderEnvelope;

    fn envelope(units: Vec<UnitId>) -> PlayerCommand {
        PlayerCommand {
            seq: 1,
            order: OrderEnvelope {
                order: "unknown:order".into(),
                units,
                target: OrderTarget::None,
                queue: turts_net_protocol::QueueMode::Replace,
                group: 0,
            },
        }
    }

    fn empty_catalog() -> ContentCatalog {
        ContentCatalog {
            resources: Default::default(),
            units: Default::default(),
            nodes: Default::default(),
            production: Default::default(),
            recipes: Default::default(),
            technologies: Default::default(),
            orders: Default::default(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: Default::default(),
        }
    }

    #[test]
    fn k4_caps_non_move_envelopes_before_catalog_or_simulation_work() {
        let command = envelope((0..=MAX_ORDER_UNITS as u32).map(UnitId).collect());
        let result = to_sim(SlotId(0), &command, &empty_catalog())
            .expect_err("the universal envelope cap is 4096 units");
        assert_eq!(result.envelope, None);
        assert_eq!(result.refused.len(), MAX_ORDER_UNITS + 1);
        assert!(
            result
                .refused
                .iter()
                .all(|refusal| refusal.reason == CommandRejectReason::TooManyUnits)
        );
    }

    #[test]
    fn empty_move_reaches_the_simulation_validator() {
        let mut catalog = empty_catalog();
        let id = ContentId::parse("test:move").unwrap();
        catalog.orders.insert(
            id.clone(),
            turts_mod_api::OrderDef {
                id: id.local.clone(),
                name: "Move".into(),
                behaviour: Behaviour::Move,
                target: turts_mod_api::TargetKind::Point,
                fire: None,
                engage: None,
                icon: String::new(),
                hotkey: String::new(),
                cursor: None,
                card: None,
                units: Vec::new(),
                description: String::new(),
            },
        );
        let command = PlayerCommand {
            seq: 2,
            order: OrderEnvelope {
                order: id.to_string(),
                units: Vec::new(),
                target: OrderTarget::Point(turts_net_protocol::Pos { x: 2.0, y: 3.0 }),
                queue: turts_net_protocol::QueueMode::Replace,
                group: 0,
            },
        };
        let converted = to_sim(SlotId(0), &command, &catalog).expect("Move maps to sim");
        assert_eq!(converted.len(), 1);
        assert!(matches!(
            &converted[0].kind,
            CommandKind::Order(order) if order.units.is_empty()
        ));
    }

    #[test]
    fn wire_reason_covers_every_sim_reason() {
        use RejectReason as R;

        let cases = [
            (R::EmptyCommand, CommandRejectReason::EmptyCommand),
            (R::TooManyUnits, CommandRejectReason::TooManyUnits),
            (R::DuplicateUnit, CommandRejectReason::DuplicateUnit),
            (R::UnknownUnit, CommandRejectReason::UnknownUnit),
            (R::NotOwner, CommandRejectReason::NotOwner),
            (R::NotMobile, CommandRejectReason::NotMobile),
            (R::TargetNotFinite, CommandRejectReason::TargetNotFinite),
            (R::TargetOutsideMap, CommandRejectReason::TargetOutsideMap),
            (R::MissingCapability, CommandRejectReason::MissingCapability),
            (R::UnknownNode, CommandRejectReason::UnknownNode),
            (R::IncompatibleItem, CommandRejectReason::IncompatibleItem),
            (R::NodeDepleted, CommandRejectReason::NodeDepleted),
            (R::UnknownRecipe, CommandRejectReason::UnknownRecipe),
            (R::RecipeNotEnabled, CommandRejectReason::RecipeNotEnabled),
            (R::QueueFull, CommandRejectReason::QueueFull),
            (
                R::InsufficientBalance,
                CommandRejectReason::InsufficientBalance,
            ),
            (R::UnknownBalance, CommandRejectReason::InsufficientBalance),
            (R::InsufficientItems, CommandRejectReason::InsufficientItems),
            (R::UnknownInventory, CommandRejectReason::InsufficientItems),
            (R::InvalidAmount, CommandRejectReason::InsufficientItems),
            (R::UnknownJob, CommandRejectReason::UnknownJob),
            (
                R::RefundDestinationFull,
                CommandRejectReason::RefundDestinationFull,
            ),
            (
                R::RefundDestinationOverflow,
                CommandRejectReason::RefundDestinationOverflow,
            ),
            (
                R::ArithmeticOverflow,
                CommandRejectReason::ArithmeticOverflow,
            ),
            (R::IdExhausted, CommandRejectReason::IdExhausted),
            (R::BadSequence, CommandRejectReason::BadSequence),
            (R::NotRemovable, CommandRejectReason::NotRemovable),
            (R::NotToggleable, CommandRejectReason::NotToggleable),
            (
                R::PrerequisitesMissing,
                CommandRejectReason::PrerequisitesMissing,
            ),
            (R::AlreadyResearched, CommandRejectReason::AlreadyResearched),
            (
                R::ResearchInProgress,
                CommandRejectReason::ResearchInProgress,
            ),
            (R::IntegrityViolation, CommandRejectReason::Internal),
        ];
        for (reason, expected) in cases {
            assert_eq!(wire_reason(reason), expected, "{}", reason.as_str());
        }
    }
}
