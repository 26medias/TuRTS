//! HUD3 schema/accessor contracts. The first implementation intentionally returns empty
//! values so these assertions provide semantic red signatures before validation is added.
use std::collections::BTreeMap;

use turts_mod_runtime::{
    archive::PackageArchive, content_mc_validation::check_locked_set_caps, pack::pack_files,
};

fn archive_with_order(order: &str) -> PackageArchive {
    archive_with_order_rules(order, r#"(queue_depth:16,smart:[])"#)
}

fn archive_with_order_rules(order: &str, rules: &str) -> PackageArchive {
    let manifest = r#"(id:"hud",version:"1.0.0",kind:Game,engine_api:"^0.2.3",dependencies:{},exports:(units:[],maps:["maps/map.ron"],visuals:[],resources:[],nodes:[],production:[],orders:["orders/rush.ron","orders/rules.ron"],interface:["interface/main.ron"],input:["input/main.ron"],assets:["assets/main.ron"]),context:Shared,capabilities:[],license:"test")"#;
    let mut files = BTreeMap::new();
    files.insert("manifest.ron".into(), manifest.as_bytes().to_vec());
    files.insert(
        "maps/map.ron".into(),
        br#"(id:"map",size:(x:10.0,y:10.0),placements:[],nodes:[],slot_economy:[])"#.to_vec(),
    );
    files.insert("orders/rush.ron".into(), order.as_bytes().to_vec());
    files.insert("orders/rules.ron".into(), rules.as_bytes().to_vec());
    files.insert(
        "interface/main.ron".into(),
        br##"(
            id:"main",
            root:[
                (id:"card",size:(w:Px(64.0),h:Px(24.0)),widget:Button(icon:Some(Item(Icon)),label:None,action:Item(Action),hotkey_label:true)),
                (id:"minimap",widget:MinimapView(frame:"#ffffff")),
            ],
            tooltip_style:(background:"#000000",colour:"#ffffff",font:None,size:12),
            cursors:{},
            camera:(pitch:0.7,yaw:0.0,field_of_view:60.0,minimum:10.0,maximum:100.0,start:Point(0.0,0.0),height:40.0,margin:0.0,pan_speed:1.0,edge_band:true),
            light:None,
            slot_colours:[],
            world:(ground:"#000000",grid:"#ffffff",selection:"#ffffff",health:"#00ff00",range:"#ffffff",show_range:true,tooltip:(background:"#000000",colour:"#ffffff",font:None,size:12),health_bar:(width:32,height:4,background:"#000000",health:"#00ff00")),
            alerts:[
                (id:"damaged",trigger:OwnUnitDamaged,sound:None,text:"Damaged",minimap_ping:true,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"destroyed",trigger:OwnUnitDestroyed,sound:None,text:"Destroyed",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"production",trigger:ProductionComplete,sound:None,text:"Production",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"research",trigger:ResearchComplete,sound:None,text:"Research",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"ready",trigger:ResearchReady,sound:None,text:"Ready",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"full",trigger:ExtractorFull,sound:None,text:"Full",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"depleted",trigger:ExtractorDepleted,sound:None,text:"Depleted",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"blocked",trigger:OutputBlocked,sound:None,text:"Blocked",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"ammo",trigger:OutOfAmmo,sound:None,text:"Ammo",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"inserter",trigger:InserterWaiting(5.0),sound:None,text:"Inserter",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
                (id:"gatherer",trigger:GathererWaiting(5.0),sound:None,text:"Gatherer",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:2.0),
            ],
            reasons:{},
            units:{},
            selection_priority:[],
            guide:[
                (id:"factory",title:"Factory",body:"Make products.",widgets:[]),
                (id:"research",title:"Research",body:"Research upgrades.",widgets:[]),
                (id:"pause",title:"Pause",body:"Pause a machine.",widgets:[]),
                (id:"drone",title:"Drone",body:"The drone player gathers.",widgets:[]),
                (id:"controls",title:"Controls",body:"Current controls.",widgets:[(id:"binding-list",widget:List(source:Bindings,direction:Vertical,gap:2,max_items:32,item:(id:"binding-row",size:(w:Fill,h:Fit),widget:Text(text:Item(Name),font:None,size:12,colour:"#ffffff",align:Left,wrap:false))))]),
            ],
        )"##
            .to_vec(),
    );
    files.insert(
        "input/main.ron".into(),
        br#"(actions:[(name:"rush",default:[Key(code:"ShiftLeft",mods:[])]),(name:"cancel",default:[Key(code:"KeyC",mods:[])])],engine_overrides:[(name:"add_modifier",bindings:[Key(code:"ShiftLeft",mods:[])]),(name:"queue_modifier",bindings:[Key(code:"ShiftLeft",mods:[])])])"#
            .to_vec(),
    );
    files.insert(
        "assets/main.ron".into(),
        br#"(images:[(id:"rush-icon",path:"assets/icon.png",nine_slice:None)],fonts:[],sounds:[],cursors:[])"#.to_vec(),
    );
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&32u32.to_be_bytes());
    png.extend_from_slice(&32u32.to_be_bytes());
    png.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0]);
    files.insert("assets/icon.png".into(), png);
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}

fn archive() -> PackageArchive {
    archive_with_order(
        r#"(id:"rush",name:"Rush",behaviour:Attack,target:Unit,fire:None,engage:None,icon:"rush-icon",hotkey:"rush",cursor:None,card:Some((row:0,column:0)),units:["hud:trooper"],description:"Rush the target")"#,
    )
}

fn image_archive(id: &str, count: usize) -> PackageArchive {
    let exported = (0..count)
        .map(|index| {
            format!("(id:\"img-{index}\",path:\"assets/img-{index}.png\",nine_slice:None)")
        })
        .collect::<Vec<_>>()
        .join(",");
    let manifest = format!(
        "(id:\"{id}\",version:\"1.0.0\",kind:Game,engine_api:\"^0.2.3\",dependencies:{{}},exports:(units:[],maps:[\"maps/map.ron\"],visuals:[],resources:[],nodes:[],production:[],orders:[],interface:[],input:[],assets:[\"assets/main.ron\"]),context:Shared,capabilities:[],license:\"test\")"
    );
    let assets = format!("(images:[{exported}],fonts:[],sounds:[],cursors:[])");
    let mut files = BTreeMap::from([
        ("manifest.ron".into(), manifest.into_bytes()),
        (
            "maps/map.ron".into(),
            br#"(id:"map",size:(x:10.0,y:10.0),placements:[],nodes:[],slot_economy:[])"#.to_vec(),
        ),
        ("assets/main.ron".into(), assets.into_bytes()),
    ]);
    for index in 0..count {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&2048u32.to_be_bytes());
        png.extend_from_slice(&2048u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0]);
        files.insert(format!("assets/img-{index}.png"), png);
    }
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}

#[test]
fn archive_accessors_parse_all_api_023_export_groups() {
    let archive = archive();
    assert_eq!(
        archive.orders().unwrap().len(),
        1,
        "orders accessor skipped OrderDef"
    );
    assert_eq!(
        archive.interfaces().unwrap().len(),
        1,
        "interface accessor skipped InterfaceDef"
    );
    assert_eq!(
        archive.input_maps().unwrap().len(),
        1,
        "input accessor skipped InputMapDef"
    );
    assert_eq!(
        archive.assets().unwrap().len(),
        1,
        "assets accessor skipped AssetsDef"
    );
    assert_eq!(archive.order_rules().unwrap().len(), 1);
}

#[test]
fn validator_accepts_modifier_keys_and_minimap_color_frame_with_defaults() {
    let archive = archive_with_order(
        r#"(id:"rush",name:"Rush",behaviour:Move,target:Point,fire:None,engage:None,icon:"rush-icon",hotkey:"rush",cursor:None,card:None,units:[],description:"Move")"#,
    );
    let catalog = archive
        .content()
        .expect("modifier physical key and MinimapView frame color are valid");
    assert_eq!(catalog.orders.len(), 1);
    let rush = catalog.orders.keys().next().unwrap();
    assert_eq!(catalog.source_files.get(rush).unwrap(), "orders/rush.ron");
}

#[test]
fn validator_rejects_cancel_production_with_point_target_semantically() {
    let archive = archive_with_order(
        r#"(id:"cancel",name:"Cancel",behaviour:CancelProduction,target:Point,fire:None,engage:None,icon:"rush-icon",hotkey:"cancel",cursor:None,card:None,units:["hud:trooper"],description:"Cancel a job")"#,
    );
    let error = archive.content().unwrap_err();
    assert_eq!(
        error.to_string(),
        "hud/orders/rush.ron: target: CancelProduction requires Job, got Point"
    );
}

#[test]
fn validator_rejects_queue_depth_zero_semantically() {
    let archive = archive_with_order_rules(
        r#"(id:"rush",name:"Rush",behaviour:Attack,target:Unit,fire:None,engage:None,icon:"rush-icon",hotkey:"rush",cursor:None,card:None,units:[],description:"Rush the target")"#,
        r#"(queue_depth:0,smart:[])"#,
    );
    let error = archive.content().unwrap_err();
    assert_eq!(
        error.to_string(),
        "hud/orders/rules.ron: queue_depth: must be between 1 and 255, got 0"
    );
}

#[test]
fn order_rules_accessor_preserves_queue_depth_for_range_validation() {
    let archive = archive_with_order_rules(
        r#"(id:"rush",name:"Rush",behaviour:Attack,target:Unit,fire:None,engage:None,icon:"rush-icon",hotkey:"rush",cursor:None,card:None,units:[],description:"Rush the target")"#,
        r#"(queue_depth:256,smart:[])"#,
    );
    assert_eq!(archive.order_rules().unwrap()[0].1.queue_depth, 256);
}

#[test]
fn order_rules_accessor_does_not_silently_skip_unknown_fields() {
    let archive = archive_with_order_rules(
        r#"(id:"rush",name:"Rush",behaviour:Attack,target:Unit,fire:None,engage:None,icon:"rush-icon",hotkey:"rush",cursor:None,card:None,units:[],description:"Rush the target")"#,
        r#"(queue_dept:16,smart:[])"#,
    );
    assert!(
        archive
            .order_rules()
            .unwrap_err()
            .to_string()
            .contains("hud/orders/rules.ron: unknown field `queue_dept`")
    );
}

#[test]
fn order_rules_detection_ignores_quoted_and_commented_field_names() {
    let archive = archive_with_order(
        r#"(id:"rush",name:"Rush",behaviour:Attack,target:Unit,fire:None,engage:None,icon:"rush-icon",hotkey:"rush",cursor:None,card:None,units:[],description:"queue_depth: marker",bogus:true)
            // smart: marker"#,
    );
    let error = archive.orders().unwrap_err().to_string();
    assert!(error.contains("unknown field `bogus`"), "{error}");
}

#[test]
fn locked_set_image_cap_uses_the_aggregate_decoded_png_bytes() {
    let first = image_archive("first", 9);
    let second = image_archive("second", 9);
    assert!(
        check_locked_set_caps(&[&first, &second]).is_err(),
        "18 decoded 2048x2048 RGBA images exceed the 256 MiB locked-set cap"
    );
}
