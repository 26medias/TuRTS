mod common;

use bevy_app::App;
use common::{
    accounting::{AccountingOracle, ExpectedOperation, advance_accounted_tick, trail},
    scenario,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};
use turts_mod_api::{
    ContentCatalog, CostSource, PaymentPolicy, ResolvedDropOffEffect, ResourceKind,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::{load_package, pack_checked};
use turts_sim::{
    Applied, Command, CommandKind, NodeId, SlotId, apply_command, audit_state,
    bootstrap::InstalledCatalog,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn generator() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let path = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/m2_variants");
    assert!(
        path.is_file(),
        "build crates/turts_checks example m2_variants first"
    );
    path
}
fn generate(src: &Path, out: &Path, mode: &str, payment: &str) -> Value {
    let seed = "1414";
    let output = ProcessCommand::new(generator())
        .args([
            "--source",
            src.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--seed",
            seed,
            "--mode",
            mode,
            "--payment",
            payment,
        ])
        .output()
        .expect("run variant generator");
    assert!(
        output.status.success(),
        "generator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&fs::read(out.join("mapping.json")).unwrap()).unwrap()
}
fn package_dir(out: &Path, mapping: &Value) -> PathBuf {
    let (_, p) = mapping["packages"]
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    out.join(p["new"].as_str().unwrap())
}
fn load(dir: &Path) -> (PackageArchive, App) {
    let bytes = pack_checked(dir).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive).unwrap();
    trail(&mut app).enable();
    (archive, app)
}
fn validate(catalog: &ContentCatalog, expected: PaymentPolicy, mode: &str) {
    assert!(!catalog.units.is_empty());
    assert!(!catalog.resources.is_empty());
    assert!(!catalog.nodes.is_empty());
    let balance_resources = catalog
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Balance)
        .count();
    let item_resources = catalog
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Item)
        .count();
    match mode {
        "balance" => {
            assert_eq!((balance_resources, item_resources), (1, 1));
            assert!(
                catalog
                    .production
                    .values()
                    .flat_map(|r| &r.costs)
                    .all(|c| c.source == CostSource::SlotBalance)
            );
            let converts = catalog.units.values().filter_map(|u| u.drop_off.as_ref()).flat_map(|d| &d.rules)
                .filter(|r| matches!(r.effect, ResolvedDropOffEffect::Convert { per_item, .. } if per_item > 0)).count();
            assert!(converts > 0, "balance role has a positive conversion sink");
        }
        "physical" => {
            assert_eq!(item_resources, 1);
            assert!(
                catalog
                    .production
                    .values()
                    .flat_map(|r| &r.costs)
                    .all(|c| c.source == CostSource::ProducerInventory)
            );
            let stores = catalog
                .units
                .values()
                .filter_map(|u| u.drop_off.as_ref())
                .flat_map(|d| &d.rules)
                .filter(|r| matches!(r.effect, ResolvedDropOffEffect::Store))
                .count();
            assert!(stores > 0, "physical role stores extracted cargo");
        }
        other => panic!("unexpected role {other}"),
    }
    for (id, recipe) in &catalog.production {
        assert_eq!(&recipe.id, id);
        assert_eq!(recipe.payment, expected);
        assert!(!recipe.costs.is_empty());
        assert!(recipe.cancel_refunds.queued.denominator > 0);
        assert!(recipe.cancel_refunds.running.denominator > 0);
        assert!(recipe.cancel_refunds.output_blocked.denominator > 0);
    }
    for (id, unit) in &catalog.units {
        if let Some(inv) = &unit.inventory {
            assert!(inv.capacity > 0, "{id} capacity");
            assert!(
                inv.accepts
                    .iter()
                    .all(|item| catalog.resources.contains_key(item))
            );
        }
        if let Some(prod) = &unit.producer {
            assert!(!prod.recipes.is_empty());
            assert!(
                prod.recipes
                    .iter()
                    .all(|recipe| catalog.production.contains_key(recipe))
            );
        }
    }
}
fn item_amount(
    state: &turts_sim::AuditState,
    unit: u32,
    resource: &turts_mod_api::ContentId,
) -> u64 {
    state
        .economy
        .inventories
        .iter()
        .find(|i| i.unit == unit)
        .and_then(|i| i.items.iter().find(|x| x.resource == *resource))
        .map_or(0, |item| item.on_hand)
}

fn prove_local_store(package_dir: &Path, payment: PaymentPolicy) {
    let catalog = {
        let (_, app) = load(package_dir);
        app.world().resource::<InstalledCatalog>().0.clone()
    };
    let (producer_id, unit) = catalog
        .units
        .iter()
        .find(|(_, u)| u.producer.is_some())
        .unwrap();
    let producer_id = producer_id.to_string();
    let recipe_id = unit
        .producer
        .as_ref()
        .unwrap()
        .recipes
        .iter()
        .min_by_key(|r| {
            catalog.production[*r]
                .costs
                .iter()
                .map(|c| c.amount)
                .sum::<u64>()
        })
        .unwrap()
        .clone();
    let recipe = &catalog.production[&recipe_id];
    assert_eq!(recipe.payment, payment);
    assert_eq!(
        recipe.costs.len(),
        1,
        "physical locality scenario requires one item cost"
    );
    let cost = &recipe.costs[0];
    assert_eq!(cost.source, CostSource::ProducerInventory);
    let item = cost.resource.clone();

    let map_path = fs::read_dir(package_dir.join("maps"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let map_text = fs::read_to_string(&map_path).unwrap();
    let placements = map_text.find("placements: [").unwrap();
    let start = map_text[placements..].find("        (").unwrap() + placements;
    let mut depth = 0i32;
    let mut end = None;
    for (offset, ch) in map_text[start..].char_indices() {
        if ch == '(' {
            depth += 1;
        }
        if ch == ')' {
            depth -= 1;
            if depth == 0 {
                end = Some(start + offset + 1);
                break;
            }
        }
    }
    let mut depot_b = map_text[start..end.unwrap()].to_owned();
    let pos_start = depot_b.find("pos: ").unwrap() + "pos: ".len();
    let mut pos_depth = 0i32;
    let mut pos_end = None;
    for (offset, ch) in depot_b[pos_start..].char_indices() {
        if ch == '(' {
            pos_depth += 1;
        }
        if ch == ')' {
            pos_depth -= 1;
            if pos_depth == 0 {
                pos_end = Some(pos_start + offset + 1);
                break;
            }
        }
    }
    depot_b.replace_range(pos_start..pos_end.unwrap(), "(x: 4.0, y: 40.0)");
    let contents_start = depot_b.find("contents: ").unwrap() + "contents: ".len();
    let mut list_depth = 0i32;
    let mut contents_end = None;
    for (offset, ch) in depot_b[contents_start..].char_indices() {
        if ch == '[' {
            list_depth += 1;
        }
        if ch == ']' {
            list_depth -= 1;
            if list_depth == 0 {
                contents_end = Some(contents_start + offset + 1);
                break;
            }
        }
    }
    depot_b.replace_range(contents_start..contents_end.unwrap(), "[]");
    let mut changed = map_text;
    let node_marker = "    ],\n    nodes: [";
    let insert_at = changed.find(node_marker).unwrap();
    changed.insert_str(insert_at, &format!("{depot_b},\n"));
    fs::write(&map_path, changed).unwrap();

    let bytes = pack_checked(package_dir).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let producers: Vec<_> = initial
        .units
        .iter()
        .filter(|u| u.content_id == producer_id && u.owner_slot == 0)
        .map(|u| u.unit_id)
        .collect();
    assert!(producers.len() >= 2);
    let from = producers[0];
    let to = *producers.last().unwrap();
    assert!(item_amount(&initial, from, &item) >= cost.amount);
    assert_eq!(item_amount(&initial, to, &item), 0);
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let rejected = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Produce {
            producer_id: to,
            recipe_id: recipe_id.clone(),
        },
    };
    assert!(
        apply_trace_command(&mut app, &mut oracle, rejected, &mut Vec::new())
            .outcome
            .is_err()
    );
    assert_eq!(
        item_amount(&audit_state(app.world_mut()), to, &item),
        0,
        "empty B rejects without creating local stock"
    );

    let before = audit_state(app.world_mut());
    let result = turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        from,
        to,
        item.clone(),
        cost.amount,
    );
    assert_eq!(result, Ok(()));
    let after = audit_state(app.world_mut());
    let records = trail(&mut app).drain_operations();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].before, before,
        "transfer capture has the live pre-state"
    );
    assert_eq!(
        records[0].after, after,
        "transfer capture has the live post-state"
    );
    assert!(
        trail(&mut app).valid,
        "audit capture remains valid at transfer boundary"
    );
    oracle
        .observe(
            &ExpectedOperation::InventoryTransfer {
                slot: 0,
                from,
                to,
                item: item.clone(),
                amount: cost.amount,
            },
            &before,
            &after,
            &records[0].events,
            None,
        )
        .unwrap();
    assert_eq!(
        item_amount(&after, from, &item),
        item_amount(&initial, from, &item) - cost.amount
    );
    assert_eq!(item_amount(&after, to, &item), cost.amount);

    let produce = Command {
        slot: SlotId(0),
        seq: 2,
        kind: CommandKind::Produce {
            producer_id: to,
            recipe_id,
        },
    };
    assert!(matches!(
        apply_trace_command(&mut app, &mut oracle, produce, &mut Vec::new()).outcome,
        Ok(Applied { job_id: Some(_) })
    ));
    let queued = audit_state(app.world_mut());
    assert_eq!(
        item_amount(&queued, from, &item),
        item_amount(&after, from, &item),
        "A remains at post-transfer stock"
    );
    if payment == PaymentPolicy::ChargeOnQueue {
        assert_eq!(
            item_amount(&queued, to, &item),
            0,
            "queue charge consumes only B stock"
        );
    } else {
        assert_eq!(
            item_amount(&queued, to, &item),
            cost.amount,
            "reserve policy holds only B stock before start"
        );
    }
    let (ops, _) = advance_accounted_tick(&mut app, &mut oracle, 1);
    assert!(!ops.is_empty());
    let started = audit_state(app.world_mut());
    assert_eq!(
        item_amount(&started, from, &item),
        item_amount(&after, from, &item),
        "A remains unchanged after B starts"
    );
    assert_eq!(
        item_amount(&started, to, &item),
        0,
        "B consumes exactly its local recipe cost"
    );
}

fn read_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy();
                out.insert(
                    rel.replace(std::path::MAIN_SEPARATOR, "/"),
                    fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn valid_policy_role_swaps_follow_data() {
    let repo = root();
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("m2-variants-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let mut count = 0;
    for package in ["base", "haul", "cedar"] {
        for mode in ["balance", "physical"] {
            for payment in ["charge", "reserve"] {
                let out = scratch.join(format!("{package}-{mode}-{payment}"));
                let map = generate(&repo.join("mods").join(package), &out, mode, payment);
                assert_eq!(map["mode"], mode);
                assert_eq!(map["payment"], payment);
                let (archive, mut app) = load(&package_dir(&out, &map));
                assert_eq!(archive.manifest.version, "0.2.0");
                let policy = if payment == "charge" {
                    PaymentPolicy::ChargeOnQueue
                } else {
                    PaymentPolicy::ReserveThenCharge
                };
                validate(&app.world().resource::<InstalledCatalog>().0, policy, mode);
                let state = audit_state(app.world_mut());
                assert!(state.resources.allocators);
                for account in &state.economy.balances {
                    let resource = app
                        .world()
                        .resource::<InstalledCatalog>()
                        .0
                        .resources
                        .get(&account.resource)
                        .expect("balance row references an installed resource");
                    assert_eq!(
                        resource.kind,
                        ResourceKind::Balance,
                        "item resource must never be installed as a slot balance account: {}",
                        account.resource
                    );
                }
                if mode == "physical" {
                    assert!(
                        state
                            .economy
                            .balances
                            .iter()
                            .all(|b| b.on_hand == 0 && b.reserved == 0),
                        "physical role has no funded/default balance accounts: {package}/{payment}"
                    );
                }
                if mode == "physical" {
                    prove_local_store(&package_dir(&out, &map), policy);
                }
                let progress = run_package_trace(&package_dir(&out, &map)).1;
                assert!(
                    progress.0 > 0
                        && progress.1 > 0
                        && progress.2 > 0
                        && progress.3 > 0
                        && progress.4 > 0
                        && progress.5 > 0,
                    "each variant must extract, deliver, pay, cancel/refund, and complete before passing: {package}/{mode}/{payment} {progress:?}"
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 12);
}

#[test]
fn acme_remains_a_separate_empty_economy_public_load() {
    let (archive, mut app) = load(&root().join("mods/acme"));
    assert_eq!(archive.manifest.id, "acme");
    let s = audit_state(app.world_mut());
    assert!(s.economy.balances.is_empty());
    assert!(s.economy.inventories.is_empty());
    assert!(s.economy.queues.is_empty());
    assert!(s.nodes.is_empty());
}

#[test]
fn independent_cedar_packs_and_conserves() {
    let cedar = root().join("mods/cedar");
    let files = read_tree(&cedar);
    assert_eq!(files.len(), 10);
    assert!(!cedar.join("Cargo.toml").exists());
    assert!(!cedar.join("src").exists());
    let (archive, app) = load(&cedar);
    assert_eq!(archive.manifest.id, "cedar");
    let c = &app.world().resource::<InstalledCatalog>().0;
    assert_eq!(
        (
            c.units.len(),
            c.nodes.len(),
            c.resources.len(),
            c.production.len()
        ),
        (2, 1, 1, 1)
    );
    let progress = run_package_trace(&cedar).1;
    assert!(
        progress.0 > 0
            && progress.1 > 0
            && progress.2 > 0
            && progress.3 > 0
            && progress.4 > 0
            && progress.5 > 0,
        "independent Cedar must extract, deliver, pay, cancel/refund, and complete: {progress:?}"
    );
}

#[test]
fn declared_conversion_coefficient_is_used_by_public_trace() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("m2-conversion-coefficient-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let output = scratch.join("base-balance-charge");
    let mapping = generate(&root().join("mods/base"), &output, "balance", "charge");
    let package = package_dir(&output, &mapping);
    let mut conversions = Vec::new();
    for (path, bytes) in read_tree(&package) {
        let mut text = String::from_utf8(bytes).unwrap();
        if text.contains("effect: Convert") {
            assert_eq!(
                text.matches("per_item: 1").count(),
                1,
                "one authored conversion coefficient in {path}"
            );
            text = text.replace("per_item: 1", "per_item: 2");
            fs::write(package.join(&path), text).unwrap();
            conversions.push(path);
        }
    }
    assert_eq!(
        conversions.len(),
        1,
        "one public conversion fixture was transformed"
    );
    let (_, app) = load(&package);
    let catalog = &app.world().resource::<InstalledCatalog>().0;
    assert!(
        catalog
            .units
            .values()
            .filter_map(|u| u.drop_off.as_ref())
            .flat_map(|d| &d.rules)
            .any(|r| matches!(r.effect, ResolvedDropOffEffect::Convert { per_item: 2, .. }))
    );
    let progress = run_package_trace(&package).1;
    assert!(
        progress.0 > 0
            && progress.1 > 0
            && progress.2 > 0
            && progress.3 > 0
            && progress.4 > 0
            && progress.5 > 0,
        "the modified coefficient package extracts, delivers, pays, cancels/refunds, and completes: {progress:?}"
    );
}

#[test]
fn public_loading_path_has_no_native_hooks() {
    let repo = root();
    let mut pending = vec![repo.join("mods")];
    while let Some(dir) = pending.pop() {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                pending.push(p);
            } else {
                assert_ne!(p.file_name().unwrap(), "Cargo.toml");
                assert_ne!(p.extension().and_then(|x| x.to_str()), Some("rs"));
            }
        }
    }
    let api = fs::read_to_string(repo.join("crates/turts_pack/src/lib.rs")).unwrap();
    assert!(api.contains("pub fn load_package"));
    assert!(api.contains("install_package(world, staged)"));
    assert!(!api.contains("World::spawn"));
}

fn write_package_tree(dir: &Path, files: &BTreeMap<String, Vec<u8>>) {
    for (relative, bytes) in files {
        let path = dir.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

fn assert_cedar_missing_ref(error: &turts_mod_api::ModError) {
    assert_eq!(error.package, "cedar");
    assert_eq!(error.file.as_deref(), Some("units/worker.ron"));
    assert_eq!(error.field_path, "inventory.accepts[0]");
    assert!(error.message.contains("cedar:missing"), "{}", error.message);
    assert_eq!(error.allowed, Some(vec!["cedar:wood".to_owned()]));
}

#[test]
fn cedar_broken_reference_reports_public_package_file_and_field() {
    let mut files = read_tree(&root().join("mods/cedar"));
    let worker = files.get_mut("units/worker.ron").unwrap();
    let original = String::from_utf8(worker.clone()).unwrap();
    assert!(original.matches("cedar:wood").count() >= 1);
    *worker = original.replace("cedar:wood", "cedar:missing").into_bytes();
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("cedar-broken-public-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    write_package_tree(&scratch, &files);

    let checked = pack_checked(&scratch).expect_err("public checked packer rejects broken ref");
    assert_cedar_missing_ref(&checked);

    let bytes = pack_dir(&scratch).expect("raw public packer serializes authored files");
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = scenario(20);
    let loaded =
        load_package(app.world_mut(), &archive).expect_err("public loader rejects broken ref");
    assert_cedar_missing_ref(&loaded);
    assert_eq!(
        checked, loaded,
        "checked packing and package loading report the same provenance"
    );
    fs::remove_dir_all(&scratch).unwrap();
}

fn apply_trace_command(
    app: &mut App,
    oracle: &mut AccountingOracle,
    command: Command,
    trace: &mut Vec<Value>,
) -> turts_sim::CommandResult {
    let before = audit_state(app.world_mut());
    assert_eq!(
        &before,
        oracle.modeled_state(),
        "command begins at model state"
    );
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let records = trail(app).drain_operations();
    assert_eq!(records.len(), 1, "one captured operation per command");
    assert_eq!(
        records[0].before, before,
        "operation capture has the live command pre-state"
    );
    assert_eq!(
        records[0].after, after,
        "operation capture has the live command post-state"
    );
    let capture = trail(app);
    assert!(
        capture.valid,
        "audit capture remains valid at every command boundary"
    );
    let command_record = capture
        .commands
        .iter()
        .find(|r| r.operation_id == records[0].operation_id)
        .expect("command audit is associated with its logical operation");
    assert_eq!(command_record.input, command);
    assert_eq!(command_record.result, result);
    assert_eq!(command_record.before, before);
    assert_eq!(command_record.after, after);
    oracle
        .observe(
            &ExpectedOperation::Command(command.clone()),
            &before,
            &after,
            &records[0].events,
            Some(&result),
        )
        .unwrap_or_else(|e| panic!("command {}: {e}", command.seq));
    trace.push(json!({"command": command, "result": result, "operation": records}));
    result
}

type TraceTotals = (u128, u128, u128, u128, u64, u64);
type PackageTrace = (Vec<Value>, TraceTotals);
type CanonicalMap = BTreeMap<String, String>;
type CanonicalMaps = (
    CanonicalMap,
    CanonicalMap,
    CanonicalMap,
    CanonicalMap,
    CanonicalMap,
);

fn run_package_trace(dir: &Path) -> PackageTrace {
    let (_, mut app) = load(dir);
    let initial = audit_state(app.world_mut());
    let catalog = app.world().resource::<InstalledCatalog>().0.clone();
    let worker_type = catalog
        .units
        .iter()
        .find(|(_, u)| u.gatherer.is_some())
        .unwrap()
        .0
        .to_string();
    let (producer_type, recipe) = catalog
        .units
        .iter()
        .find_map(|(id, u)| {
            u.producer
                .as_ref()
                .map(|p| (id.to_string(), p.recipes.clone()))
        })
        .and_then(|(id, recipes)| {
            recipes
                .into_iter()
                .min_by_key(|r| {
                    catalog.production[r]
                        .costs
                        .iter()
                        .map(|c| c.amount)
                        .sum::<u64>()
                })
                .map(|r| (id, r))
        })
        .unwrap();
    let worker = initial
        .units
        .iter()
        .find(|u| u.content_id == worker_type && u.owner_slot == 0)
        .unwrap()
        .unit_id;
    let producer = initial
        .units
        .iter()
        .find(|u| u.content_id == producer_type && u.owner_slot == 0)
        .unwrap()
        .unit_id;
    let pos = initial
        .raw_units
        .iter()
        .find(|u| u.unit_id == worker)
        .unwrap()
        .pos
        .clone();
    let node = initial
        .nodes
        .iter()
        .min_by(|a, b| {
            let da = (a.pos.x - pos.x).powi(2) + (a.pos.y - pos.y).powi(2);
            let db = (b.pos.x - pos.x).powi(2) + (b.pos.y - pos.y).powi(2);
            da.total_cmp(&db)
        })
        .unwrap()
        .id
        .0;
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let mut trace = vec![json!({"initial":initial})];
    let gather = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Gather {
            unit_id: worker,
            node_id: NodeId(node),
        },
    };
    assert_eq!(
        apply_trace_command(&mut app, &mut oracle, gather, &mut trace).outcome,
        Ok(Applied { job_id: None })
    );
    let mut tick = 0;
    while oracle.delivery == 0 && tick < 2000 {
        tick += 1;
        let (ops, record) = advance_accounted_tick(&mut app, &mut oracle, tick);
        trace.push(json!({"operations":ops,"tick":record}));
    }
    assert!(
        oracle.extraction > 0 && oracle.delivery > 0,
        "gather extracts and delivers before equality"
    );
    let queued = Command {
        slot: SlotId(0),
        seq: 2,
        kind: CommandKind::Produce {
            producer_id: producer,
            recipe_id: recipe.clone(),
        },
    };
    let queued_result = apply_trace_command(&mut app, &mut oracle, queued, &mut trace);
    let Ok(Applied {
        job_id: Some(queued_job),
    }) = queued_result.outcome
    else {
        panic!("queued recipe applied: {queued_result:?}")
    };
    let queued_cancel = Command {
        slot: SlotId(0),
        seq: 3,
        kind: CommandKind::CancelProduction {
            producer_id: producer,
            job_id: queued_job,
        },
    };
    assert_eq!(
        apply_trace_command(&mut app, &mut oracle, queued_cancel, &mut trace).outcome,
        Ok(Applied { job_id: None })
    );
    let first = Command {
        slot: SlotId(0),
        seq: 4,
        kind: CommandKind::Produce {
            producer_id: producer,
            recipe_id: recipe.clone(),
        },
    };
    let result = apply_trace_command(&mut app, &mut oracle, first, &mut trace);
    let Ok(Applied { job_id: Some(job) }) = result.outcome else {
        panic!("first recipe applied: {result:?}")
    };
    tick += 1;
    let (ops, record) = advance_accounted_tick(&mut app, &mut oracle, tick);
    trace.push(json!({"operations":ops,"tick":record}));
    let cancel = Command {
        slot: SlotId(0),
        seq: 5,
        kind: CommandKind::CancelProduction {
            producer_id: producer,
            job_id: job,
        },
    };
    assert_eq!(
        apply_trace_command(&mut app, &mut oracle, cancel, &mut trace).outcome,
        Ok(Applied { job_id: None })
    );
    let next = Command {
        slot: SlotId(0),
        seq: 6,
        kind: CommandKind::Produce {
            producer_id: producer,
            recipe_id: recipe,
        },
    };
    let result = apply_trace_command(&mut app, &mut oracle, next, &mut trace);
    assert!(matches!(result.outcome, Ok(Applied { job_id: Some(_) })));
    let mut completed = false;
    while oracle.outputs == 0 && tick < 2000 {
        tick += 1;
        let (ops, record) = advance_accounted_tick(&mut app, &mut oracle, tick);
        trace.push(json!({"operations":ops,"tick":record}));
        completed = true;
    }
    assert!(
        completed && oracle.outputs > 0,
        "second job completes within 2,000 ticks"
    );
    assert!(oracle.gross_charge > 0 && oracle.refunds > 0 && oracle.cancellations > 0);
    oracle
        .assert_command_records(&trail(&mut app).commands)
        .unwrap();
    oracle.assert_positive_progress().unwrap();
    assert!(
        trail(&mut app).valid,
        "audit capture remains valid after the complete trace"
    );
    (
        trace,
        (
            oracle.extraction,
            oracle.delivery,
            oracle.gross_charge,
            oracle.refunds,
            oracle.cancellations,
            oracle.outputs,
        ),
    )
}

fn inverses(mapping: &Value) -> CanonicalMaps {
    fn put(m: &mut BTreeMap<String, String>, new: &str, old: &str) {
        if let Some(prior) = m.insert(new.to_owned(), old.to_owned()) {
            assert_eq!(prior, old, "ambiguous canonical mapping for {new}");
        }
    }
    let mut packages = BTreeMap::new();
    let mut locals = BTreeMap::new();
    let mut full = BTreeMap::new();
    let mut names = BTreeMap::new();
    let mut paths = BTreeMap::new();
    for p in mapping["packages"].as_object().unwrap().values() {
        put(
            &mut packages,
            p["new"].as_str().unwrap(),
            p["old"].as_str().unwrap(),
        );
    }
    for d in mapping["definitions"].as_array().unwrap() {
        put(
            &mut locals,
            d["new_local_id"].as_str().unwrap(),
            d["old_local_id"].as_str().unwrap(),
        );
        if !d["new_full_id"].is_null() {
            put(
                &mut full,
                d["new_full_id"].as_str().unwrap(),
                d["old_full_id"].as_str().unwrap(),
            );
        }
    }
    for n in mapping["names"].as_array().unwrap() {
        put(
            &mut names,
            n["new"].as_str().unwrap(),
            n["old"].as_str().unwrap(),
        );
    }
    for p in mapping["paths"].as_array().unwrap() {
        for (new, old) in [
            ("new_export_path", "old_export_path"),
            ("new_package_path", "old_package_path"),
            ("new_archive_path", "old_archive_path"),
        ] {
            put(
                &mut paths,
                p[new].as_str().unwrap(),
                p[old].as_str().unwrap(),
            );
        }
    }
    (packages, locals, full, names, paths)
}

fn normalize(value: &mut Value, m: &CanonicalMaps, inherited: &BTreeMap<u64, String>) {
    if let Value::Object(o) = value {
        let mut types = inherited.clone();
        if let Some(table) = o.get("unit_types").and_then(Value::as_array) {
            types.clear();
            for entry in table {
                let id = entry["type_id"].as_u64().unwrap();
                let content = entry["content_id"].as_str().unwrap();
                let canonical = m.2.get(content).unwrap_or(&content.to_owned()).clone();
                assert!(types.insert(id, canonical).is_none());
            }
            assert!(!types.is_empty());
        }
        for (key, child) in o.iter_mut() {
            if (key == "unit_type" || key == "type_id") && child.is_number() {
                let id = child.as_u64().unwrap();
                *child = Value::String(format!(
                    "unit-type:{}",
                    types.get(&id).expect("dense type has content mapping")
                ));
            } else if let Value::String(text) = child {
                let map = match key.as_str() {
                    "0" | "package" => m.0.get(text),
                    "local" | "id" => m.1.get(text),
                    "name" => m.3.get(text),
                    k if k.contains("path") || k.contains("file") => m.4.get(text),
                    _ => m.2.get(text).or_else(|| m.3.get(text)),
                };
                if let Some(replacement) = map {
                    *text = replacement.clone();
                }
            } else {
                normalize(child, m, &types);
            }
            if key == "unit_types"
                && let Value::Array(items) = child
            {
                items.sort_by_key(|item| item["content_id"].to_string());
            }
            if matches!(
                key.as_str(),
                "compiled_durations" | "balances" | "inventories" | "orders" | "queues"
            ) && let Value::Array(items) = child
            {
                items.sort_by_key(Value::to_string);
            }
            if key == "catalog" {
                sort_catalog(child);
            }
        }
    } else if let Value::Array(a) = value {
        for child in a {
            normalize(child, m, inherited);
        }
    }
}

fn sort_catalog(value: &mut Value) {
    let Some(catalog) = value.as_object_mut() else {
        return;
    };
    for key in ["units", "nodes", "recipes"] {
        if let Some(Value::Array(items)) = catalog.get_mut(key) {
            items.sort_by_key(|item| item["id"].to_string());
        }
    }
    if let Some(Value::Array(items)) = catalog.get_mut("resource_kinds") {
        items.sort_by_key(Value::to_string);
    }
}

fn first_diff(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(aa), Value::Object(bb)) => {
            if aa.len() != bb.len() {
                return Some(format!(
                    "{path}: object key count {} != {}",
                    aa.len(),
                    bb.len()
                ));
            }
            for (key, av) in aa {
                let Some(bv) = bb.get(key) else {
                    return Some(format!("{path}.{key}: missing right key"));
                };
                if let Some(diff) = first_diff(av, bv, &format!("{path}.{key}")) {
                    return Some(diff);
                }
            }
            None
        }
        (Value::Array(aa), Value::Array(bb)) => {
            if aa.len() != bb.len() {
                return Some(format!("{path}: array length {} != {}", aa.len(), bb.len()));
            }
            aa.iter()
                .zip(bb)
                .enumerate()
                .find_map(|(i, (x, y))| first_diff(x, y, &format!("{path}[{i}]")))
        }
        _ if a == b => None,
        _ => Some(format!("{path}: {a} != {b}")),
    }
}

#[test]
fn renamed_base_and_physical_clone_have_positive_equal_traces() {
    let scratch =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("m2-trace-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let base_renamed = scratch.join("base-renamed");
    let base_mapping = generate(&root().join("mods/base"), &base_renamed, "rename", "charge");
    let base_b = package_dir(&base_renamed, &base_mapping);
    let (base_a_trace, base_a_progress) = run_package_trace(&root().join("mods/base"));
    let (mut base_b_trace, base_b_progress) = run_package_trace(&base_b);
    assert!(
        base_a_progress.0 > 0
            && base_a_progress.1 > 0
            && base_a_progress.2 > 0
            && base_a_progress.3 > 0
            && base_a_progress.4 > 0
            && base_a_progress.5 > 0
    );
    assert_eq!(
        base_a_progress, base_b_progress,
        "base rename retains positive economy outcomes"
    );
    let base_inverse = inverses(&base_mapping);
    let identity = (
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let mut base_a_trace = base_a_trace;
    for v in &mut base_a_trace {
        normalize(v, &identity, &BTreeMap::new());
    }
    for v in &mut base_b_trace {
        normalize(v, &base_inverse, &BTreeMap::new());
    }
    assert_eq!(
        first_diff(&json!(base_a_trace), &json!(base_b_trace), "base trace"),
        None,
        "base pure rename canonicalizes only mapped content and dense type handles"
    );
    let physical = scratch.join("physical");
    let role = generate(&root().join("mods/cedar"), &physical, "physical", "reserve");
    let a = package_dir(&physical, &role);
    let renamed = scratch.join("renamed");
    let mapping = generate(&a, &renamed, "rename", "reserve");
    let b = package_dir(&renamed, &mapping);
    let (ta, pa) = run_package_trace(&a);
    let (mut tb, pb) = run_package_trace(&b);
    assert!(
        pa.0 > 0 && pa.1 > 0 && pa.2 > 0 && pa.3 > 0 && pa.4 > 0 && pa.5 > 0,
        "extract, deliver, pay, refund, cancel, complete before equality: {pa:?}"
    );
    assert_eq!(pa, pb, "rename retains all progress amounts and outcomes");
    let inverse = inverses(&mapping);
    let identity = (
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let mut ta = ta;
    for v in &mut ta {
        normalize(v, &identity, &BTreeMap::new());
    }
    for v in &mut tb {
        normalize(v, &inverse, &BTreeMap::new());
    }
    assert_eq!(
        first_diff(&json!(ta), &json!(tb), "trace"),
        None,
        "canonical traces retain all fields and allocation/tick identity"
    );
}
