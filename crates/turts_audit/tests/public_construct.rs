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
    catalog.production.insert(id("build"), recipe);
    let mut order = catalog.orders[&id("order_produce")].clone();
    order.id = "order_construct".into();
    order.behaviour = Behaviour::Construct;
    order.target = TargetKind::ContentAtPoint;
    catalog.orders.insert(id("order_construct"), order.clone());
    let mut app = turts_sim::headless_app(20);
    let staged = turts_sim::stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: "public_construct/map".into(),
            value: archive.maps().unwrap().remove(0).1,
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
                behaviour: Behaviour::Construct,
                target_kind: TargetKind::ContentAtPoint,
                policy: Some(policy),
                units: vec![1],
                target: OrderTargetSim::ContentAtPoint {
                    content: id("build"),
                    point: turts_sim::Vec2::new(20.0, 20.0),
                },
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: order.units,
                queue_depth: definition.queue_depth,
            })),
        },
    );
    assert!(result.outcome.is_ok(), "ordinary Construct: {result:?}");
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
