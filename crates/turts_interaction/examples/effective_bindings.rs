use std::path::Path;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

pub fn effective_bindings(package: &Path) -> Result<serde_json::Value, String> {
    let bytes = pack_dir(package).map_err(|e| e.to_string())?;
    let archive = PackageArchive::parse(&bytes).map_err(|e| e.to_string())?;
    let package_id = archive.manifest.id.clone();
    let mut input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    for (_, input) in archive.input_maps().map_err(|e| e.to_string())? {
        input_map.actions.extend(input.actions);
        input_map.engine_overrides.extend(input.engine_overrides);
    }
    let actions = turts_interaction::input_map::effective_actions_with_runtime(
        &input_map,
        &std::collections::BTreeMap::new(),
    );
    let mut orders = std::collections::BTreeMap::new();
    for (_, order) in archive.orders().map_err(|e| e.to_string())? {
        let order_id = if order.id.contains(':') {
            order.id.clone()
        } else {
            format!("{package_id}:{}", order.id)
        };
        if let Some(bindings) = actions.get(&order.hotkey) {
            orders.insert(order_id, bindings.clone());
        }
    }
    let actions = actions
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                serde_json::to_value(value).expect("bindings serialize"),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let orders = orders
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                serde_json::to_value(value).expect("bindings serialize"),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    Ok(serde_json::json!({"actions": actions, "orders": orders}))
}

#[allow(dead_code)] // Included as a module by the integration test for helper-level coverage.
pub fn main() {
    let package = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    match effective_bindings(&package) {
        Ok(value) => println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("JSON value serializes")
        ),
        Err(error) => {
            eprintln!("effective_bindings: {error}");
            std::process::exit(1);
        }
    }
}
