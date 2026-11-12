//! Design Player path step 5 (PT1): while the Proving Lab holds fewer than 10 shells, the research
//! entry tooltip says "Needs 10 Shell (n/10)" and the research key. The key label comes from the
//! effective bindings: a runtime rebinding changes it, so no literal key can pass.

use std::{collections::BTreeMap, fs, path::PathBuf, sync::OnceLock};

use bevy::platform::time::Instant;
use turts_client::{
    content::ClientContent,
    replica::{Replica, ReplicaContext, SnapshotOutcome},
    widgets::sources::{self, SourceContext},
};
use turts_mod_api::{Binding, InterfaceDef, ItemField, Manifest, Primitive, Source, Value, Widget};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    BalanceState, EconomyState, Facing, InventoryPort, InventoryState, Pos, QueueState, SlotId,
    Snapshot, UnitId, UnitSnapshot,
};

const LAB: u32 = 7;

fn forge_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge")
}

fn content() -> &'static ClientContent {
    static CONTENT: OnceLock<ClientContent> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let archive = PackageArchive::parse(&pack_dir(&forge_dir()).expect("pack forge"))
            .expect("parse forge");
        ClientContent::from_archives(&[archive]).expect("resolve forge")
    })
}

fn interface() -> InterfaceDef {
    let manifest: Manifest = ron::from_str(
        &fs::read_to_string(forge_dir().join("manifest.ron")).expect("read Forge manifest"),
    )
    .expect("parse Forge manifest");
    let path = forge_dir().join(&manifest.exports.interface[0]);
    ron::from_str(&fs::read_to_string(path).expect("read Forge interface"))
        .expect("parse Forge interface")
}

fn find_widget<'a>(widgets: &'a [Widget], id: &str) -> Option<&'a Widget> {
    widgets.iter().find_map(|widget| {
        if widget.id == id {
            return Some(widget);
        }
        match &widget.widget {
            Primitive::Box { children, .. } => find_widget(children, id),
            _ => None,
        }
    })
}

fn lab_replica(shells: u64) -> Replica {
    let snapshot = Snapshot {
        tick: 10,
        units: vec![UnitSnapshot {
            unit_id: UnitId(LAB),
            content_id: "forge:lab".into(),
            owner_slot: SlotId(0),
            facing: Some(Facing::PosX),
            pos: Pos { x: 7.0, y: 0.0 },
        }],
        nodes: vec![],
        economy: EconomyState {
            balances: vec![],
            inventories: vec![InventoryState {
                unit: UnitId(LAB),
                port: InventoryPort::Main,
                items: vec![BalanceState {
                    resource: "forge:shell".into(),
                    on_hand: shells,
                    reserved: 0,
                }],
            }],
            orders: vec![],
            queues: vec![QueueState {
                producer: UnitId(LAB),
                enabled: true,
                jobs: vec![],
            }],
        },
        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
        unit_orders: Vec::new(),
    };
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content().clone(), 0));
    assert_eq!(
        replica.apply_snapshot(&snapshot, Instant::now()),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    replica
}

/// The research entry tooltip text as the live HUD binds it (widgets/tree.rs:377-383).
fn research_tooltip(shells: u64, bindings: &BTreeMap<String, Vec<Binding>>) -> String {
    let interface = interface();
    let entries = find_widget(&interface.root, "research_entries")
        .expect("Forge interface has the research_entries list");
    let tooltip = entries
        .tooltip
        .as_ref()
        .expect("the research entry list declares a tooltip");
    let replica = lab_replica(shells);
    let content = content();
    let selection = [LAB];
    let groups = BTreeMap::new();
    let context = SourceContext {
        binding_labels: None,
        replica: &replica,
        content,
        selection: &selection,
        owner_slot: 0,
        interface: Some(&interface),
        interface_package: Some("forge"),
        control_groups: &groups,
        alerts: &[],
        bindings,
        input_map: &content.input_map,
        guide: &interface.guide,
        guide_open: false,
        last_notice: None,
        core: turts_client::widgets::sources::CoreFeed::empty(),
    };
    let rows = sources::read_source(&Source::BuildMenu, context).items;
    let row = rows
        .iter()
        .find(|row| {
            row.get(ItemField::Name).and_then(serde_json::Value::as_str) == Some("Hardened Shells")
        })
        .expect("the lab build menu lists Hardened Shells");
    assert!(
        matches!(tooltip, Value::Format(..)),
        "the research tooltip is a Format value"
    );
    sources::display_text_value(&sources::read_value_for_item(tooltip, context, Some(row)))
}

#[test]
fn research_tooltip_names_reason_and_effective_key() {
    let text = research_tooltip(3, &BTreeMap::new());
    assert!(
        text.contains("Needs 10 Shell (3/10)"),
        "the tooltip must name the missing shells: {text:?}"
    );
    assert!(
        text.contains("[R]"),
        "the tooltip must name the effective research key: {text:?}"
    );
}

#[test]
fn research_tooltip_follows_a_runtime_rebinding() {
    let bindings = BTreeMap::from([(
        "key_research".to_owned(),
        vec![Binding::Key {
            code: "KeyQ".into(),
            mods: vec![],
        }],
    )]);
    let text = research_tooltip(0, &bindings);
    assert!(
        text.contains("Needs 10 Shell (0/10)") && text.contains("[Q]") && !text.contains("[R]"),
        "a rebinding must change the key in the tooltip: {text:?}"
    );
}
