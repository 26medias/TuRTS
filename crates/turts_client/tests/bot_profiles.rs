//! Aggregate package validation must precede host-only controller activation.
use std::{collections::BTreeMap, path::Path};
use turts_client::content::ClientContent;
use turts_mod_runtime::{
    archive::PackageArchive,
    pack::{pack_dir, pack_files},
};
fn library() -> PackageArchive {
    let files=BTreeMap::from([
        ("manifest.ron".into(), br#"(id:"support",version:"0.1.0",kind:Library,engine_api:"^0.2.15",dependencies:{},exports:(units:["unit.ron"],maps:[],visuals:["visual.ron"],resources:[],nodes:[],production:[],bots:["bot.ron"]),context:Shared,capabilities:[],license:"MIT")"#.to_vec()),
        ("unit.ron".into(),br#"(id:"marker",name:"Marker",footprint:Circle(radius:1.0),movement:None,inventory:None,gatherer:None,drop_off:None,producer:None)"#.to_vec()),
        ("visual.ron".into(),br#"(target:Unit("support:marker"),shape:Cuboid(x:1.0,y:1.0,z:1.0))"#.to_vec()),
        ("bot.ron".into(),br#"(id:"custom",name:"Custom host policy",decision_ms:1000,start_when:MatchRunning,knowledge:(),controllers:[(key:"custom",factory:"host_extension:custom",config:Map({"mode":String("standby")}),quota:(queries:1,candidates:1,rows:1,proposals:1))])"#.to_vec()),
    ]);
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}
#[test]
fn libraries_load_custom_profiles_without_installing_or_requiring_host_factories() {
    let game = PackageArchive::parse(
        &pack_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish")).unwrap(),
    )
    .unwrap();
    let lib = library();
    let content = ClientContent::from_archives(&[game, lib]).unwrap();
    assert_eq!(content.catalog.bots.len(), 2);
    let custom = &content.catalog.bots[&turts_mod_api::ContentId::parse("support:custom").unwrap()];
    assert_eq!(custom.controllers[0].factory, "host_extension:custom");
}
#[test]
fn aggregate_validation_resolves_profile_roles_after_all_gameplay_archives() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    fn visit(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                visit(base, &p, out)
            } else {
                out.insert(
                    p.strip_prefix(base).unwrap().to_str().unwrap().into(),
                    std::fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(&root, &root, &mut files);
    let profile = String::from_utf8(files["bots/opponent.ron"].clone())
        .unwrap()
        .replace(
            "roles: [",
            "roles: [(key: \"external\", units: [\"support:marker\"]),",
        );
    files.insert("bots/opponent.ron".into(), profile.into_bytes());
    let game = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert!(
        game.content().is_err(),
        "a standalone load must not silently skip unresolved bot references"
    );
    let content = ClientContent::from_archives(&[game, library()]).unwrap();
    assert_eq!(content.catalog.bots.len(), 2);
}
