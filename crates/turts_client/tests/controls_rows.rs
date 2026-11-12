//! Player path step 14 (PT1): a first-time player finds "Stop" in the controls list and rebinds
//! it. A row of a mod action takes the name of the order that has the action as its hotkey
//! (`mods/forge/orders/order_stop.ron`: `key_stop` gives "Stop selected units"). The rows show
//! the mod actions first, then camera and selection, then the control groups (RUN131 decision).

use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

use turts_client::{
    content::ClientContent,
    replica::Replica,
    widgets::sources::{self, SourceContext},
};
use turts_mod_api::{ItemField, Source};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

fn forge() -> &'static ClientContent {
    static CONTENT: OnceLock<ClientContent> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
        let archive =
            PackageArchive::parse(&pack_dir(&dir).expect("pack forge")).expect("parse forge");
        ClientContent::from_archives(&[archive]).expect("resolve forge")
    })
}

/// The `Bindings` rows (guide sheet 5) as (label, key label) pairs, in source order.
fn binding_rows() -> Vec<(String, String)> {
    let content = forge();
    let replica = Replica::new(20);
    let groups = BTreeMap::new();
    let bindings = BTreeMap::new();
    let context = SourceContext {
        binding_labels: None,
        replica: &replica,
        content,
        selection: &[],
        owner_slot: 0,
        interface: None,
        interface_package: Some("forge"),
        control_groups: &groups,
        alerts: &[],
        bindings: &bindings,
        input_map: &content.input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
    };
    sources::read_source(&Source::Bindings, context)
        .items
        .iter()
        .map(|row| {
            let text = |field| {
                row.get(field)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            (text(ItemField::Name), text(ItemField::Title))
        })
        .collect()
}

fn is_group(label: &str) -> bool {
    ["Assign group ", "Select group ", "Add to group "]
        .iter()
        .any(|prefix| label.starts_with(prefix))
}

#[test]
fn a_mod_action_row_takes_the_name_of_its_order() {
    let rows = binding_rows();
    assert!(
        rows.contains(&("Stop selected units".to_owned(), "S".to_owned())),
        "key_stop shows the order name: {rows:?}"
    );
    assert!(
        rows.iter().all(|(label, _)| !label.starts_with("Key ")),
        "no row shows an action id: {rows:?}"
    );
    assert!(
        rows.iter().any(|(label, _)| label == "Pan camera left"),
        "an engine action keeps its readable label: {rows:?}"
    );
}

#[test]
fn mod_actions_come_first_then_camera_and_selection_then_groups() {
    let rows = binding_rows();
    let labels = rows
        .iter()
        .map(|(label, _)| label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels.len(), 51, "{labels:?}");
    let mut orders = forge()
        .catalog
        .orders
        .values()
        .map(|order| order.name.as_str())
        .collect::<Vec<_>>();
    orders.sort_unstable();
    let mut first = labels[..orders.len()].to_vec();
    first.sort_unstable();
    assert_eq!(first, orders, "the mod action rows come first: {labels:?}");
    let first_group = labels.iter().position(|label| is_group(label)).unwrap();
    assert!(
        labels[first_group..].iter().all(|label| is_group(label)),
        "the group rows come last: {labels:?}"
    );
    assert!(
        first_group > orders.len(),
        "camera and selection rows come between: {labels:?}"
    );
}
