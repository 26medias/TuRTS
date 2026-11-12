//! Orders domain: the order definition that an order command carries (a catalog rule).
//!
//! The server builds each order command from a package order definition. The sim checks the unit
//! list of the command only when it is not empty, so a server that builds a wrong definition
//! lets an order reach a unit type that the mod does not list. The rules compare the logged
//! command with the immutable catalog snapshot of the log. They need no model of the simulation.
//! They hold the cover of `validate_order_policy` of the deleted replay validator (A1c).
//!
//! Rules (each message names the field):
//! - orders: the catalog of the log holds exactly one definition for the order of the command, and
//!   the capability entries of that definition name each unit one time.
//! - orders: the behaviour, the target kind, the unit list (as a set) and the queue depth of the
//!   command equal those of the catalog definition.
//! - orders: the command holds its captured policy, and the id, the behaviour, the target kind,
//!   the fire policy, the engage policy, the unit list (as a set), the queue depth and the unit
//!   capabilities (as a set) of the policy equal those of the catalog definition.
//! - orders: the target kind of the policy fits its behaviour.
//! - orders: for a Remove order, the `removable` capability of the unit type of each subject
//!   equals the `by_command` removal policy of the loaded unit.

use turts_mod_api::{Behaviour, TargetKind};
use turts_sim::{CommandKind, OrderUnitCapabilitiesAudit};

use crate::CommandLog;

fn sorted_units(units: &[String]) -> Vec<&String> {
    let mut sorted: Vec<&String> = units.iter().collect();
    sorted.sort();
    sorted
}

fn sorted_capabilities(
    capabilities: &[OrderUnitCapabilitiesAudit],
) -> Vec<&OrderUnitCapabilitiesAudit> {
    let mut sorted: Vec<&OrderUnitCapabilitiesAudit> = capabilities.iter().collect();
    sorted.sort_by(|left, right| left.unit.cmp(&right.unit));
    sorted
}

pub(crate) fn check_command(command: &CommandLog) -> Result<(), String> {
    let CommandKind::Order(order) = &command.input.kind else {
        return Ok(());
    };
    let id = command.operation_id;
    let differs = |field: &str| {
        Err(format!(
            "orders: order command {id} differs from its immutable catalog definition in {field}"
        ))
    };
    let catalog =
        command.before.catalog.as_ref().ok_or_else(|| {
            format!("orders: order command {id} has no immutable content catalog")
        })?;
    let mut definitions = catalog
        .order_definitions
        .iter()
        .filter(|definition| definition.id == order.order);
    let (Some(definition), None) = (definitions.next(), definitions.next()) else {
        return Err(format!(
            "orders: order command {id} has a missing or duplicate catalog definition"
        ));
    };
    let capabilities = sorted_capabilities(&definition.unit_capabilities);
    if capabilities
        .windows(2)
        .any(|pair| pair[0].unit == pair[1].unit)
    {
        return Err(format!(
            "orders: order command {id} has duplicate capability entries"
        ));
    }
    if order.behaviour == Behaviour::Remove {
        for subject in &order.units {
            let Some(content) = command
                .before
                .economy
                .units
                .iter()
                .find(|unit| unit.unit_id == *subject)
                .map(|unit| unit.content_id.as_str())
            else {
                continue;
            };
            let Some(loaded) = catalog
                .units
                .iter()
                .find(|unit| unit.id.to_string() == content)
            else {
                continue;
            };
            let catalog_removable = loaded
                .removal
                .as_ref()
                .is_some_and(|policy| policy.by_command);
            let captured_removable = capabilities
                .iter()
                .find(|capability| capability.unit == content)
                .is_some_and(|capability| capability.removable);
            if catalog_removable != captured_removable {
                return Err(format!(
                    "orders: order command {id} has Remove capability inconsistent with the loaded policy of unit {subject}"
                ));
            }
        }
    }
    if order.behaviour != definition.behaviour {
        return differs("behaviour");
    }
    if order.target_kind != definition.target {
        return differs("target kind");
    }
    if sorted_units(&order.allowed_units) != sorted_units(&definition.units) {
        return differs("allowed units");
    }
    if order.queue_depth != definition.queue_depth {
        return differs("queue depth");
    }
    let policy = order.policy.as_ref().ok_or_else(|| {
        format!("orders: order command {id} omits its captured catalog definition")
    })?;
    if policy.id != definition.id {
        return differs("policy id");
    }
    if policy.behaviour != definition.behaviour {
        return differs("policy behaviour");
    }
    if policy.target != definition.target {
        return differs("policy target kind");
    }
    if policy.fire != definition.fire {
        return differs("policy fire");
    }
    if policy.engage != definition.engage {
        return differs("policy engage");
    }
    if sorted_units(&policy.units) != sorted_units(&definition.units) {
        return differs("policy units");
    }
    if policy.queue_depth != definition.queue_depth {
        return differs("policy queue depth");
    }
    if sorted_capabilities(&policy.unit_capabilities) != capabilities {
        return differs("policy unit capabilities");
    }
    let target_fits = match order.behaviour {
        Behaviour::Move | Behaviour::Attack => matches!(
            policy.target,
            TargetKind::Point | TargetKind::Unit | TargetKind::PointOrUnit
        ),
        Behaviour::Stop | Behaviour::Hold | Behaviour::Remove | Behaviour::SetEnabled(_) => {
            policy.target == TargetKind::None
        }
        Behaviour::Gather => policy.target == TargetKind::Node,
        Behaviour::Produce => policy.target == TargetKind::Content,
        Behaviour::CancelProduction => policy.target == TargetKind::Job,
    };
    if !target_fits {
        return Err(format!(
            "orders: order command {id} has an invalid behavior/target policy pair"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_command;
    use crate::CommandLog;
    use turts_mod_api::{Behaviour, ContentId, EngagePolicy, FirePolicy, TargetKind};
    use turts_sim::economy::views::{CatalogAudit, CatalogOrderAudit};
    use turts_sim::orders::{OrderCommand, OrderTargetSim, QueueMode};
    use turts_sim::{
        Command, CommandKind, CommandResult, OrderPolicyAudit, OrderUnitCapabilitiesAudit, SlotId,
    };

    fn capability(unit: &str, removable: bool) -> OrderUnitCapabilitiesAudit {
        OrderUnitCapabilitiesAudit {
            unit: unit.into(),
            movement: true,
            weapon: false,
            gatherer: false,
            producer: false,
            research: false,
            removable,
            toggleable: false,
        }
    }

    fn definition() -> CatalogOrderAudit {
        CatalogOrderAudit {
            id: ContentId::parse("pkg:rush").unwrap(),
            behaviour: Behaviour::Attack,
            target: TargetKind::Point,
            fire: Some(FirePolicy::Never),
            engage: Some(EngagePolicy::StopAndFire),
            units: vec!["pkg:a".into(), "pkg:b".into()],
            queue_depth: 4,
            unit_capabilities: vec![capability("pkg:a", false), capability("pkg:b", false)],
        }
    }

    /// A consistent order command and its catalog definition.
    fn command() -> CommandLog {
        let definition = definition();
        let mut app = turts_sim::headless_app(10);
        let mut before = turts_sim::ledger_audit(&turts_sim::audit_state(app.world_mut()));
        before.catalog = Some(CatalogAudit {
            resource_kinds: Vec::new(),
            nodes: Vec::new(),
            units: Vec::new(),
            recipes: Vec::new(),
            order_definitions: vec![definition.clone()],
            technologies: Vec::new(),
            item_recipes: Vec::new(),
        });
        let input = Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: definition.id.clone(),
                behaviour: definition.behaviour,
                target_kind: definition.target,
                policy: Some(OrderPolicyAudit {
                    id: definition.id.clone(),
                    behaviour: definition.behaviour,
                    target: definition.target,
                    fire: definition.fire,
                    engage: definition.engage,
                    units: definition.units.clone(),
                    queue_depth: definition.queue_depth,
                    unit_capabilities: definition.unit_capabilities.clone(),
                }),
                units: vec![1],
                target: OrderTargetSim::Point(turts_sim::Vec2::new(1.0, 1.0)),
                queue: QueueMode::Replace,
                group: 1,
                allowed_units: definition.units.clone(),
                queue_depth: definition.queue_depth,
            })),
        };
        CommandLog {
            operation_id: 7,
            operation_end_id: None,
            result: CommandResult {
                slot: input.slot,
                seq: input.seq,
                outcome: Err(turts_sim::RejectReason::EmptyCommand),
                units: Default::default(),
            },
            input,
            after: before.clone(),
            before,
        }
    }

    fn order(command: &mut CommandLog) -> &mut OrderCommand {
        let CommandKind::Order(order) = &mut command.input.kind else {
            unreachable!()
        };
        order
    }

    fn policy(command: &mut CommandLog) -> &mut OrderPolicyAudit {
        order(command).policy.as_mut().unwrap()
    }

    fn catalog_definition(command: &mut CommandLog) -> &mut CatalogOrderAudit {
        &mut command.before.catalog.as_mut().unwrap().order_definitions[0]
    }

    fn rejected(mut command: CommandLog, mutate: impl FnOnce(&mut CommandLog)) -> String {
        mutate(&mut command);
        check_command(&command).unwrap_err()
    }

    #[test]
    fn a_consistent_order_command_passes() {
        assert_eq!(check_command(&command()), Ok(()));
        // The unit lists and the capability lists are sets: their order is free.
        let mut reordered = command();
        order(&mut reordered).allowed_units.reverse();
        policy(&mut reordered).units.reverse();
        policy(&mut reordered).unit_capabilities.reverse();
        assert_eq!(check_command(&reordered), Ok(()));
    }

    /// Each field of the command, and of its captured policy, that differs from the catalog
    /// definition makes the audit red with the name of the field.
    #[test]
    fn each_field_that_differs_from_the_catalog_definition_is_named() {
        let cases: Vec<(&str, fn(&mut CommandLog))> = vec![
            ("behaviour", |c| order(c).behaviour = Behaviour::Move),
            ("target kind", |c| order(c).target_kind = TargetKind::Unit),
            ("allowed units", |c| order(c).allowed_units = Vec::new()),
            ("allowed units", |c| {
                order(c).allowed_units.push("pkg:c".into())
            }),
            ("queue depth", |c| order(c).queue_depth = 255),
            ("policy id", |c| {
                policy(c).id = ContentId::parse("pkg:other").unwrap()
            }),
            ("policy behaviour", |c| {
                policy(c).behaviour = Behaviour::Move
            }),
            ("policy target kind", |c| {
                policy(c).target = TargetKind::Unit
            }),
            ("policy fire", |c| policy(c).fire = None),
            ("policy engage", |c| policy(c).engage = None),
            ("policy units", |c| policy(c).units.pop().map(drop).unwrap()),
            ("policy queue depth", |c| policy(c).queue_depth = 5),
            ("policy unit capabilities", |c| {
                policy(c).unit_capabilities[0].weapon = true
            }),
        ];
        for (field, mutate) in cases {
            let error = rejected(command(), mutate);
            assert!(
                error.ends_with(&format!("catalog definition in {field}")),
                "{field}: {error}"
            );
        }
    }

    #[test]
    fn a_missing_or_duplicate_definition_or_catalog_is_rejected() {
        let error = rejected(command(), |c| {
            c.before.catalog.as_mut().unwrap().order_definitions.clear()
        });
        assert!(
            error.contains("missing or duplicate catalog definition"),
            "{error}"
        );
        let error = rejected(command(), |c| {
            let copy = catalog_definition(c).clone();
            c.before
                .catalog
                .as_mut()
                .unwrap()
                .order_definitions
                .push(copy);
        });
        assert!(
            error.contains("missing or duplicate catalog definition"),
            "{error}"
        );
        let error = rejected(command(), |c| c.before.catalog = None);
        assert!(error.contains("no immutable content catalog"), "{error}");
        let error = rejected(command(), |c| order(c).policy = None);
        assert!(
            error.contains("omits its captured catalog definition"),
            "{error}"
        );
        let error = rejected(command(), |c| {
            catalog_definition(c)
                .unit_capabilities
                .push(capability("pkg:a", false))
        });
        assert!(error.contains("duplicate capability entries"), "{error}");
    }

    #[test]
    fn a_behaviour_with_a_target_kind_that_does_not_fit_is_rejected() {
        // The command and the definition agree: only the pair is wrong.
        let error = rejected(command(), |c| {
            catalog_definition(c).behaviour = Behaviour::Gather;
            order(c).behaviour = Behaviour::Gather;
            policy(c).behaviour = Behaviour::Gather;
        });
        assert!(
            error.contains("invalid behavior/target policy pair"),
            "{error}"
        );
    }

    #[test]
    fn a_remove_order_needs_the_capability_of_the_loaded_unit_policy() {
        let remove = |c: &mut CommandLog, by_command: bool, removable: bool| {
            c.before.catalog.as_mut().unwrap().units.push(
                turts_sim::economy::views::CatalogUnitAudit {
                    id: ContentId::parse("pkg:a").unwrap(),
                    radius: 0.5,
                    movement_speed: None,
                    weapon_range: None,
                    weapon_targets: None,
                    removal: Some(turts_mod_api::RemovalDef {
                        by_command,
                        contents: turts_mod_api::ContentsPolicy::Destroy,
                    }),
                    machine_recipe: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: Vec::new(),
                    producer: None,
                    research: None,
                },
            );
            c.before.economy.units.push(turts_sim::UnitState {
                unit_id: 1,
                content_id: "pkg:a".into(),
                owner_slot: 0,
                facing: None,
                pos: turts_sim::Vec2::ZERO,
            });
            catalog_definition(c).behaviour = Behaviour::Remove;
            catalog_definition(c).target = TargetKind::None;
            catalog_definition(c).unit_capabilities[0].removable = removable;
            order(c).behaviour = Behaviour::Remove;
            order(c).target_kind = TargetKind::None;
            policy(c).behaviour = Behaviour::Remove;
            policy(c).target = TargetKind::None;
            policy(c).unit_capabilities[0].removable = removable;
        };
        let mut consistent = command();
        remove(&mut consistent, true, true);
        assert_eq!(check_command(&consistent), Ok(()));
        let error = rejected(command(), |c| remove(c, true, false));
        assert!(error.contains("Remove capability inconsistent"), "{error}");
    }
}
