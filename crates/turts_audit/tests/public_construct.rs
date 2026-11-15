//! Real ordinary Construct commands exercise the independent admission/state model.
use turts_audit::{EconomyLogRecord, validate_records};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    AuditTrail, Command, CommandKind, OrderCommand, OrderPolicyAudit, OrderTargetSim, QueueMode,
    SlotId,
};
fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn stream(payment: PaymentPolicy, held: bool) -> Vec<EconomyLogRecord> {
    stream_case(payment, held, false, false)
}
fn stream_case(
    payment: PaymentPolicy,
    held: bool,
    grouped: bool,
    produce: bool,
) -> Vec<EconomyLogRecord> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let mut catalog = archive.content().unwrap();
    let mut building = catalog.units[&id("worker")].clone();
    building.id = id("building");
    building.movement = None;
    building.inventory = None;
    building.gatherer = None;
    building.health = Some(HealthDef { max: 100 });
    building.placement = Some(SitePlacementDef {
        gap: 0.0,
        node_clearance: 0.0,
        in_build_area: false,
        site_health: Ratio {
            numerator: 1,
            denominator: 4,
        },
    });
    catalog.units.insert(id("building"), building);
    catalog.units.get_mut(&id("depot")).unwrap().constructor = Some(ResolvedConstructor {
        buildings: vec![id("build")],
        concurrent_sites: 1,
        queue_capacity: 3,
        progress: ConstructProgress::Autonomous,
        when_short: vec![],
    });
    let mut recipe = catalog.production[&id("hire")].clone();
    recipe.id = id("build");
    recipe.output_unit = id("building");
    recipe.duration_ms = 1000;
    recipe.payment = payment;
    if grouped {
        catalog.resources.insert(
            id("credits"),
            ResourceDef {
                id: "credits".into(),
                name: "Credits".into(),
                unit_name: "credit".into(),
                kind: ResourceKind::Balance,
            },
        );
        recipe.costs = vec![ResolvedCost {
            resource: id("credits"),
            amount: 12,
            source: CostSource::SlotBalance,
        }];
    }
    catalog.production.insert(id("build"), recipe);
    if produce {
        catalog
            .units
            .get_mut(&id("depot"))
            .unwrap()
            .producer
            .as_mut()
            .unwrap()
            .recipes
            .push(id("build"));
    }
    let mut order = catalog.orders[&id("order_produce")].clone();
    order.id = "order_construct".into();
    order.behaviour = if produce {
        Behaviour::Produce
    } else {
        Behaviour::Construct
    };
    order.target = if produce {
        TargetKind::Content
    } else {
        TargetKind::ContentAtPoint
    };
    catalog.orders.insert(id("order_construct"), order.clone());
    let mut map = archive.maps().unwrap().remove(0).1;
    if grouped {
        // The lower physical source gets the larger real allocated ID.
        map.placements = [40.0, 8.0]
            .into_iter()
            .map(|x| Placement {
                unit: id("depot").to_string(),
                owner_slot: 0,
                pos: WorldPos { x, y: 8.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            })
            .collect();
        map.nodes.clear();
        map.slot_economy = vec![SlotEconomyInit {
            slot: 0,
            balances: vec![BalanceAmount {
                balance: id("credits").to_string(),
                amount: 12,
            }],
        }];
    }
    let mut app = turts_sim::headless_app(20);
    let staged = turts_sim::stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: "public_construct/map".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    turts_sim::install_package(app.world_mut(), staged);
    app.insert_resource(turts_sim::economy::construction::ConstructionLimit(
        if held { 0 } else { 4 },
    ));
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = turts_sim::audit_state(app.world_mut());
    let definition = initial
        .catalog
        .as_ref()
        .unwrap()
        .order_definitions
        .iter()
        .find(|definition| definition.id == id("order_construct"))
        .unwrap();
    let policy = OrderPolicyAudit {
        id: definition.id.clone(),
        behaviour: definition.behaviour,
        target: definition.target,
        fire: definition.fire,
        engage: definition.engage,
        units: definition.units.clone(),
        queue_depth: definition.queue_depth,
        unit_capabilities: definition.unit_capabilities.clone(),
    };
    let result = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: id("order_construct"),
                behaviour: order.behaviour,
                target_kind: order.target,
                policy: Some(policy),
                units: if grouped {
                    initial.raw_units.iter().map(|unit| unit.unit_id).collect()
                } else {
                    vec![1]
                },
                target: if produce {
                    OrderTargetSim::Content(id("build"))
                } else {
                    OrderTargetSim::ContentAtPoint {
                        content: id("build"),
                        point: turts_sim::Vec2::new(20.0, 20.0),
                    }
                },
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: order.units,
                queue_depth: definition.queue_depth,
            })),
        },
    );
    assert!(result.outcome.is_ok(), "ordinary Construct: {result:?}");
    if grouped {
        assert_eq!(
            result.units.accepted,
            vec![2],
            "physical source wins independently of ID: {result:?}"
        );
        assert_eq!(result.units.refused.len(), 1);
    }
    let mut trail = app.world_mut().resource_mut::<AuditTrail>();
    let mut records = vec![EconomyLogRecord::Initial(initial)];
    records.extend(
        trail
            .drain_commands()
            .into_iter()
            .map(EconomyLogRecord::CommandBoundary),
    );
    records.extend(
        trail
            .drain_operations()
            .into_iter()
            .map(EconomyLogRecord::Operation),
    );
    records
}
#[test]
fn all_current_payments_and_global_cap_held_requests_validate() {
    for payment in [
        PaymentPolicy::ChargeOnQueue,
        PaymentPolicy::ReserveThenCharge,
        PaymentPolicy::ChargeOverTime,
    ] {
        for held in [false, true] {
            validate_records(&stream(payment, held))
                .unwrap_or_else(|error| panic!("{payment:?} held{held}: {error}"));
        }
    }
}
#[test]
fn altered_site_progress_origin_payment_or_unrelated_state_is_rejected() {
    let records = stream(PaymentPolicy::ChargeOverTime, false);
    for field in [
        "progress",
        "origin",
        "health",
        "position",
        "allocator",
        "balance",
    ] {
        let mut changed = records.clone();
        let operation = changed
            .iter_mut()
            .find_map(|record| {
                if let EconomyLogRecord::Operation(operation) = record {
                    Some(operation)
                } else {
                    None
                }
            })
            .unwrap();
        let site_id = *operation
            .after
            .economy
            .construction
            .sites
            .keys()
            .next()
            .unwrap();
        match field {
            "progress" => {
                operation
                    .after
                    .economy
                    .construction
                    .sites
                    .get_mut(&site_id)
                    .unwrap()
                    .done = 1
            }
            "origin" => {
                operation
                    .after
                    .economy
                    .construction
                    .sites
                    .get_mut(&site_id)
                    .unwrap()
                    .origin = Some(turts_sim::receipts::CommandMember {
                    slot: 0,
                    sequence: 99,
                    unit: 1,
                })
            }
            "health" => {
                operation
                    .after
                    .raw_healths
                    .iter_mut()
                    .find(|row| row.unit == site_id)
                    .unwrap()
                    .current += 1
            }
            "position" => {
                operation
                    .after
                    .raw_units
                    .iter_mut()
                    .find(|row| row.unit_id == site_id)
                    .unwrap()
                    .pos
                    .x += 1.0
            }
            "allocator" => {
                let (unit, node, job) = operation.after.allocators.next_values();
                operation
                    .after
                    .allocators
                    .set_next_values(unit + 1, node, job);
            }
            "balance" => {
                operation
                    .after
                    .economy
                    .inventories
                    .iter_mut()
                    .find(|row| row.unit == 1)
                    .unwrap()
                    .items[0]
                    .on_hand -= 1
            }
            _ => unreachable!(),
        }
        // Keep the command's aggregate endpoint consistent so it cannot hide the
        // incorrect per-operation transition behind a different outer snapshot.
        let after = operation.after.clone();
        for record in &mut changed {
            if let EconomyLogRecord::CommandBoundary(command) = record {
                command.after = after.clone();
            }
        }
        assert!(validate_records(&changed).is_err(), "altered {field}");
    }
}
#[test]
fn the_global_site_cap_is_immutable_and_cannot_be_used_to_fake_a_held_request() {
    let mut records = stream(PaymentPolicy::ChargeOverTime, true);
    for record in &mut records {
        match record {
            EconomyLogRecord::Initial(state) => state.construction_limit = Some(4),
            EconomyLogRecord::CommandBoundary(command) => {
                command.before.construction_limit = Some(4);
                command.after.construction_limit = Some(4);
            }
            EconomyLogRecord::Operation(operation) => {
                operation.before.construction_limit = Some(4);
                operation.after.construction_limit = Some(4);
            }
            _ => {}
        }
    }
    assert!(
        validate_records(&records)
            .unwrap_err()
            .contains("next allocated site")
    );
}

#[test]
fn construct_aggregate_job_is_the_first_placed_job_and_held_has_none() {
    for held in [false, true] {
        let mut records = stream(PaymentPolicy::ChargeOverTime, held);
        for record in &mut records {
            if let EconomyLogRecord::CommandBoundary(command) = record {
                command.result.outcome.as_mut().unwrap().job_id = if held {
                    Some(turts_sim::JobId(999))
                } else {
                    None
                };
            }
        }
        assert!(
            validate_records(&records)
                .unwrap_err()
                .contains("aggregate JobId")
        );
    }
}

#[test]
fn grouped_immediate_paid_work_validates_physical_priority_and_rejects_coherent_id_priority() {
    fn swap_sources(state: &mut turts_sim::AuditState) {
        for unit in &mut state.raw_units {
            if unit.unit_id <= 2 {
                unit.pos.x = 48.0 - unit.pos.x;
            }
        }
        for unit in &mut state.units {
            if unit.unit_id <= 2 {
                unit.pos.x = 48.0 - unit.pos.x;
            }
        }
        for unit in &mut state.economy.units {
            if unit.unit_id <= 2 {
                unit.pos.x = 48.0 - unit.pos.x;
            }
        }
    }
    for payment in [
        PaymentPolicy::ChargeOnQueue,
        PaymentPolicy::ReserveThenCharge,
    ] {
        for produce in [false, true] {
            let records = stream_case(payment, false, true, produce);
            validate_records(&records)
                .unwrap_or_else(|error| panic!("{payment:?},produce{produce}: {error}"));
            let mut bad = records.clone();
            for record in &mut bad {
                match record {
                    EconomyLogRecord::Initial(state) => swap_sources(state),
                    EconomyLogRecord::CommandBoundary(command) => {
                        swap_sources(&mut command.before);
                        swap_sources(&mut command.after);
                    }
                    EconomyLogRecord::Operation(operation) => {
                        swap_sources(&mut operation.before);
                        swap_sources(&mut operation.after);
                    }
                    _ => {}
                }
            }
            // All captured before/after positions agree. The funded request now loses physical priority.
            let error = validate_records(&bad).expect_err(
                "coherent ID-priority stream must fail independent physical scheduling",
            );
            assert!(
                error.contains("refusal for unit") || error.contains("execution priority"),
                "priority rejection must be specific: {error}"
            );
        }
    }
}
