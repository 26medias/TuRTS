//! API17 public terrain authoring. No private realization is part of a package.
use std::collections::BTreeMap;
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files, terrain::*};

fn files(package: &str, physical: bool) -> BTreeMap<String, Vec<u8>> {
    let kind = if physical { "Item" } else { "Balance" };
    let inventory = if physical {
        format!("Some((capacity:20,accepts:[\"{package}:ore\"]))")
    } else {
        "None".into()
    };
    let cargo = if physical {
        ",terrain_cargo:Some((unload_per_second:5,interaction_range:1.0))"
    } else {
        ""
    };
    let destination = if physical {
        "Inventory"
    } else {
        "SlotBalances"
    };
    let texts = [
        (
            "manifest.ron",
            format!(
                r#"(id:"{package}",version:"1.0.0",kind:Game,engine_api:"^0.2.17",dependencies:{{}},exports:(units:["units/tool.ron"],maps:["maps/field.ron"],visuals:[],resources:["resources/ore.ron","resources/credits.ron"],nodes:[],production:[],terrain_materials:["materials/clay.ron"],terrain_movement_classes:["classes/light.ron"],terrain_tools:["tools/dig.ron","tools/fill.ron"]),context:Shared,capabilities:[],license:"test")"#
            ),
        ),
        (
            "resources/ore.ron",
            format!(r#"(id:"ore",name:"Ore",unit_name:"piece",kind:{kind})"#),
        ),
        (
            "resources/credits.ron",
            r#"(id:"credits",name:"Credits",unit_name:"credit",kind:Balance)"#.into(),
        ),
        (
            "materials/clay.ron",
            format!(
                r##"(id:"clay",name:"Clay",surface_color:"#775533",hardness:2,work_per_world_cube:(numerator:4,denominator:1),natural_outputs:[(resource:"{package}:ore",per_world_cube:(numerator:4,denominator:1))],fill_inputs:[(resource:"{package}:ore",per_world_cube:(numerator:4,denominator:1)),(resource:"{package}:credits",per_world_cube:(numerator:8,denominator:1))],placed_removal:NoYield)"##
            ),
        ),
        (
            "classes/light.ron",
            r#"(id:"light",name:"Light",max_step:1,max_grade:(numerator:1,denominator:4))"#.into(),
        ),
        (
            "tools/dig.ron",
            format!(
                r#"(id:"dig",name:"Dig",operation:Excavate,work_per_second:2,reach:2.0,allowed_materials:["{package}:clay"],max_hardness:2,output_destination:{destination})"#
            ),
        ),
        (
            "tools/fill.ron",
            format!(
                r#"(id:"fill",name:"Fill",operation:Fill(material:"{package}:clay"),work_per_second:2,reach:2.0,allowed_materials:["{package}:clay"],max_hardness:2,output_destination:{destination})"#
            ),
        ),
        (
            "units/tool.ron",
            format!(
                r#"(id:"tool",name:"Tool",footprint:Circle(radius:0.25),movement:Some((speed:3.0)),inventory:{inventory},gatherer:None,drop_off:None,producer:None,terrain_class:Some("{package}:light"),terrain_tools:["{package}:dig","{package}:fill"]{cargo})"#
            ),
        ),
        (
            "maps/field.ron",
            format!(
                r#"(id:"field",size:(x:4.0,y:4.0),placements:[(unit:"{package}:tool",owner_slot:0,pos:(x:1.5,y:1.5),contents:[])],nodes:[],slot_economy:[],terrain:Some((length_scale:1000,cell_width:1000,elevation_step:250,columns:4,rows:4,chunk_width:16,initial_elevation:0,floor:-4,ceiling:8,generation:Version1(bands:[(bottom:-4,top:0,choices:[(material:"{package}:clay",weight:1)])]))))"#
            ),
        ),
    ];
    texts
        .into_iter()
        .map(|(name, text)| (name.into(), text.into_bytes()))
        .collect()
}
fn archive(files: &BTreeMap<String, Vec<u8>>) -> PackageArchive {
    PackageArchive::parse(&pack_files(files).unwrap()).unwrap()
}
fn edit(files: &mut BTreeMap<String, Vec<u8>>, path: &str, before: &str, after: &str) {
    let text = String::from_utf8(files[path].clone()).unwrap();
    assert!(text.contains(before));
    files.insert(path.into(), text.replacen(before, after, 1).into_bytes());
}
#[test]
fn all_visible_terrain_identity_fits_the_required_atomic_frame() {
    let a = archive(&files("terrain_fixture", true));
    let catalog = a.content().unwrap();
    let (file, mut map) = a.maps().unwrap().remove(0);
    assert!(map.visibility.is_none());
    let remaining = MAX_VISIBILITY_MAP_ID_BYTES - a.id().0.len() - 1;
    map.id = "x".repeat(remaining);
    assert!(validate_terrain_map(&a.id(), &file, &map, &catalog).is_ok());
    map.id.push('x');
    let error = validate_terrain_map(&a.id(), &file, &map, &catalog).unwrap_err();
    assert_eq!(error.field, "id");
    map.terrain = None;
    assert_eq!(
        validate_terrain_map(&a.id(), &file, &map, &catalog).unwrap(),
        None
    );
}

#[test]
fn two_public_packages_resolve_physical_and_balance_tools_without_private_geology() {
    for (name, physical) in [("quarry_fixture", true), ("renamed_badlands", false)] {
        let archive = archive(&files(name, physical));
        let catalog = archive.content().unwrap();
        assert_eq!(catalog.terrain_materials.len(), 1);
        assert_eq!(catalog.terrain_tools.len(), 2);
        let (file, map) = archive.maps().unwrap().remove(0);
        let resolved = validate_terrain_map(&archive.id(), &file, &map, &catalog)
            .unwrap()
            .unwrap();
        assert_eq!(
            resolved.generation[0].choices[0].material,
            ContentId::parse(&format!("{name}:clay")).unwrap()
        );
        assert_eq!(
            (resolved.columns, resolved.rows, resolved.initial_elevation),
            (4, 4, 0)
        );
        assert_eq!(
            catalog
                .units
                .values()
                .next()
                .unwrap()
                .terrain_cargo
                .is_some(),
            physical
        );
    }
}
#[test]
fn written_api17_fields_and_explicit_nulls_fail_under_api16() {
    for (path, before, after) in [
        (
            "manifest.ron",
            ",terrain_materials:[\"materials/clay.ron\"],terrain_movement_classes:[\"classes/light.ron\"],terrain_tools:[\"tools/dig.ron\",\"tools/fill.ron\"]",
            ",terrain_materials:[]",
        ),
        (
            "units/tool.ron",
            ",terrain_class:Some(\"terrain_fixture:light\"),terrain_tools:[\"terrain_fixture:dig\",\"terrain_fixture:fill\"]",
            ",terrain_class:None",
        ),
    ] {
        let mut f = files("terrain_fixture", false);
        edit(&mut f, "manifest.ron", "^0.2.17", "^0.2.16");
        edit(&mut f, path, before, after);
        let error = archive(&f).content().unwrap_err().to_string();
        assert!(error.contains("needs engine api 0.2.17"), "{error}");
    }
    // Each new unit/map field gets a source-located refusal even when it writes its default.
    for field in [
        "terrain_class:None",
        "terrain_foundation:None",
        "terrain_cargo:None",
        "terrain_tools:[]",
    ] {
        let mut f = files("terrain_fixture", false);
        edit(&mut f, "manifest.ron", "^0.2.17", "^0.2.16");
        edit(
            &mut f,
            "manifest.ron",
            ",terrain_materials:[\"materials/clay.ron\"],terrain_movement_classes:[\"classes/light.ron\"],terrain_tools:[\"tools/dig.ron\",\"tools/fill.ron\"]",
            "",
        );
        edit(
            &mut f,
            "maps/field.ron",
            ",terrain:Some((length_scale:1000,cell_width:1000,elevation_step:250,columns:4,rows:4,chunk_width:16,initial_elevation:0,floor:-4,ceiling:8,generation:Version1(bands:[(bottom:-4,top:0,choices:[(material:\"terrain_fixture:clay\",weight:1)])])))",
            "",
        );
        edit(
            &mut f,
            "units/tool.ron",
            ",terrain_class:Some(\"terrain_fixture:light\"),terrain_tools:[\"terrain_fixture:dig\",\"terrain_fixture:fill\"]",
            &format!(",{field}"),
        );
        let error = archive(&f).content().unwrap_err().to_string();
        assert!(error.contains("units/tool.ron"), "{error}");
        assert!(error.contains("needs engine api 0.2.17"), "{error}");
    }
}
#[test]
fn private_seed_and_realized_column_fields_are_not_public_schema() {
    for private in [
        "private_seed:1",
        "seed:1",
        "columns_actual:[]",
        "actual_columns:[]",
    ] {
        let mut f = files("terrain_fixture", false);
        edit(
            &mut f,
            "maps/field.ron",
            "length_scale:1000",
            &format!("{private},length_scale:1000"),
        );
        let error = archive(&f).maps().unwrap_err().to_string();
        assert!(error.contains("unknown field"), "{error}");
    }
}
#[test]
fn invalid_storage_geometry_bands_and_fractional_prisms_fail_before_installation() {
    for (path, before, after) in [
        ("maps/field.ron", "columns:4", "columns:0"),
        ("maps/field.ron", "ceiling:8", "ceiling:2147483647"),
        ("maps/field.ron", "bottom:-4", "bottom:-3"),
        ("maps/field.ron", "top:0", "top:-1"),
        ("maps/field.ron", "weight:1", "weight:0"),
        ("maps/field.ron", "size:(x:4.0,y:4.0)", "size:(x:5.0,y:4.0)"),
        (
            "materials/clay.ron",
            "numerator:4,denominator:1",
            "numerator:1,denominator:1",
        ),
        ("materials/clay.ron", "#775533", "red"),
        (
            "tools/dig.ron",
            "terrain_fixture:clay",
            "terrain_fixture:missing",
        ),
        ("tools/dig.ron", "work_per_second:2", "work_per_second:0"),
        ("classes/light.ron", "denominator:4", "denominator:0"),
    ] {
        let mut f = files("terrain_fixture", false);
        edit(&mut f, path, before, after);
        let error = archive(&f).content().expect_err(after);
        assert!(error.file.is_some(), "{error}");
    }
}
#[test]
fn patch_overlap_cargo_compatibility_and_designation_bounds_are_checked() {
    let mut f = files("terrain_fixture", true);
    edit(
        &mut f,
        "units/tool.ron",
        ",terrain_cargo:Some((unload_per_second:5,interaction_range:1.0))",
        "",
    );
    assert!(
        archive(&f)
            .content()
            .unwrap_err()
            .field_path
            .contains("terrain_cargo")
    );
    let mut f = files("terrain_fixture", false);
    edit(
        &mut f,
        "maps/field.ron",
        "initial_elevation:0",
        "initial_elevation:0,initial_heights:[(cells:(min:(x:0,y:0),max:(x:2,y:2)),elevation:0),(cells:(min:(x:1,y:1),max:(x:3,y:3)),elevation:0)]",
    );
    assert!(
        archive(&f)
            .maps()
            .unwrap_err()
            .to_string()
            .contains("overlap")
    );
    let a = archive(&files("terrain_fixture", false));
    let c = a.content().unwrap();
    let (file, map) = a.maps().unwrap().remove(0);
    let t = validate_terrain_map(&a.id(), &file, &map, &c)
        .unwrap()
        .unwrap();
    let mut d = TerrainDesignation {
        tool: "terrain_fixture:dig".into(),
        cells: TerrainRect {
            min: TerrainCell { x: 0, y: 0 },
            max: TerrainCell { x: 4, y: 4 },
        },
        target_elevation: -4,
    };
    validate_designation(&a.id(), "command", &t, &d, &c).unwrap();
    d.cells.max.x = 5;
    assert!(validate_designation(&a.id(), "command", &t, &d, &c).is_err());
}

#[test]
fn declared_library_terrain_links_resolve_only_against_the_complete_catalog() {
    let mut library = files("terrain_lib", false);
    edit(&mut library, "manifest.ron", "kind:Game", "kind:Library");
    edit(
        &mut library,
        "manifest.ron",
        "maps:[\"maps/field.ron\"]",
        "maps:[]",
    );
    let library = archive(&library);
    let mut game = files("terrain_game", false);
    edit(
        &mut game,
        "manifest.ron",
        "dependencies:{}",
        "dependencies:{\"terrain_lib\":\"^1.0\"}",
    );
    edit(
        &mut game,
        "manifest.ron",
        "terrain_materials:[\"materials/clay.ron\"],terrain_movement_classes:[\"classes/light.ron\"],terrain_tools:[\"tools/dig.ron\",\"tools/fill.ron\"]",
        "terrain_materials:[],terrain_movement_classes:[],terrain_tools:[]",
    );
    edit(
        &mut game,
        "units/tool.ron",
        "terrain_game:light",
        "terrain_lib:light",
    );
    edit(
        &mut game,
        "units/tool.ron",
        "terrain_game:dig",
        "terrain_lib:dig",
    );
    edit(
        &mut game,
        "units/tool.ron",
        "terrain_game:fill",
        "terrain_lib:fill",
    );
    edit(
        &mut game,
        "maps/field.ron",
        "terrain_game:clay",
        "terrain_lib:clay",
    );
    let game = archive(&game);
    let mut catalog = game.content_for_catalog().unwrap();
    assert!(
        validate_terrain_catalog(&game.id(), &catalog).is_err(),
        "missing dependencies must never be installable"
    );
    let imported = library.content_for_catalog().unwrap();
    catalog.resources.extend(imported.resources);
    catalog.units.extend(imported.units);
    catalog.terrain_materials.extend(imported.terrain_materials);
    catalog
        .terrain_movement_classes
        .extend(imported.terrain_movement_classes);
    catalog.terrain_tools.extend(imported.terrain_tools);
    catalog.source_files.extend(imported.source_files);
    validate_terrain_catalog(&game.id(), &catalog).unwrap();
    let (file, map) = game.maps().unwrap().remove(0);
    let resolved = validate_terrain_map(&game.id(), &file, &map, &catalog)
        .unwrap()
        .unwrap();
    assert_eq!(
        resolved.generation[0].choices[0].material.package,
        library.id()
    );
}
