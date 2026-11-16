//! The terrain tool list binds one declared order, including at nested list depth.
use std::collections::BTreeMap;
use turts_mod_api::{Behaviour, Primitive, Source, TargetKind};
use turts_mod_runtime::{
    archive::PackageArchive, pack::pack_files, terrain::validate_terrain_tool_sources,
};
mod common;

fn package(api: &str, reference: &str) -> PackageArchive {
    let mut files = common::read_files(&common::package_dir("cedar"));
    let manifest = String::from_utf8(files["manifest.ron"].clone())
        .unwrap()
        .replace("engine_api: \"^0.2.3\"", &format!("engine_api: \"{api}\""))
        .replacen(
            "orders: [",
            "orders: [\"orders/terrain_first.ron\", \"orders/terrain_alt.ron\", ",
            1,
        );
    files.insert("manifest.ron".into(), manifest.into_bytes());
    for id in ["terrain_first", "terrain_alt"] {
        files.insert(format!("orders/{id}.ron"), format!(r#"(id:"{id}",name:"Terrain work",behaviour:TerrainWork,target:TerrainDesignation,fire:None,engage:None,icon:"icon_order_move",hotkey:"key_move",cursor:None,card:None,units:[],description:"Work designated terrain.")"#).into_bytes());
    }
    let hud = String::from_utf8(files["interface/hud.ron"].clone()).unwrap();
    assert!(hud.contains("source: BuildMenu"));
    files.insert(
        "interface/hud.ron".into(),
        hud.replacen(
            "source: BuildMenu",
            &format!("source: TerrainTools(order: {reference:?})"),
            1,
        )
        .into_bytes(),
    );
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}

#[test]
fn terrain_tools_list_has_an_explicit_qualified_order_and_keeps_the_selected_binding() {
    let archive = package("^0.2.17", "cedar:terrain_alt");
    let catalog = archive.content().unwrap();
    let (file, interface) = archive.interfaces().unwrap().remove(0);
    validate_terrain_tool_sources(&archive.id(), &file, &interface, &catalog).unwrap();
    let rows = interface
        .root
        .iter()
        .find_map(|row| match &row.widget {
            Primitive::List {
                source: Source::TerrainTools { order },
                ..
            } => Some(order),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        rows, "cedar:terrain_alt",
        "a source cannot silently bind the first TerrainWork order"
    );
    assert_eq!(
        catalog.orders[&turts_mod_api::ContentId::parse(rows).unwrap()].behaviour,
        Behaviour::TerrainWork
    );
}

#[test]
fn terrain_tools_source_api_floor_runs_before_its_invalid_order_binding() {
    let archive = package("^0.2.16", "cedar:not_declared");
    let error = archive.content().unwrap_err().to_string();
    assert!(error.contains("Source::TerrainTools"), "{error}");
    assert!(error.contains("needs engine api 0.2.17"), "{error}");
    assert!(error.contains("interface/hud.ron"), "{error}");
    assert!(
        !error.contains("unknown terrain work order"),
        "floor must precede semantic binding validation: {error}"
    );
}

#[test]
fn terrain_tool_list_rejects_missing_unqualified_undeclared_and_wrong_kind_orders() {
    for reference in [
        "cedar:missing",
        "terrain_alt",
        "foreign:terrain_alt",
        "cedar:order_move",
    ] {
        let error = package("^0.2.17", reference).content().unwrap_err();
        assert_eq!(error.file.as_deref(), Some("interface/hud.ron"));
        assert!(
            error.field_path.ends_with("source.order"),
            "{reference}: {error}"
        );
    }
    let archive = package("^0.2.17", "cedar:terrain_alt");
    let mut catalog = archive.content().unwrap();
    let (file, interface) = archive.interfaces().unwrap().remove(0);
    let id = turts_mod_api::ContentId::parse("cedar:terrain_alt").unwrap();
    catalog.orders.get_mut(&id).unwrap().target = TargetKind::Point;
    assert!(validate_terrain_tool_sources(&archive.id(), &file, &interface, &catalog).is_err());
}

#[test]
fn complete_catalog_validation_does_not_trust_a_missing_dependency_order() {
    let archive = package("^0.2.17", "cedar:terrain_alt");
    let mut catalog = archive.content().unwrap();
    let (file, mut interface) = archive.interfaces().unwrap().remove(0);
    let row = interface
        .root
        .iter_mut()
        .find(|row| matches!(row.widget, Primitive::List { .. }))
        .unwrap();
    let Primitive::List { source, .. } = &mut row.widget else {
        unreachable!()
    };
    *source = Source::TerrainTools {
        order: "library:bound_order".into(),
    };
    assert!(validate_terrain_tool_sources(&archive.id(), &file, &interface, &catalog).is_err());
    let template =
        catalog.orders[&turts_mod_api::ContentId::parse("cedar:terrain_alt").unwrap()].clone();
    catalog.orders = BTreeMap::from([(
        turts_mod_api::ContentId::parse("library:bound_order").unwrap(),
        template,
    )]);
    validate_terrain_tool_sources(&archive.id(), &file, &interface, &catalog).unwrap();
}
