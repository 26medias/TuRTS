#![allow(dead_code)]

use bevy_app::App;
use turts_mod_api::ContentId;
use turts_sim::headless_app;

pub mod accounting;

pub fn scenario(tick_hz: u16) -> App {
    headless_app(tick_hz)
}

pub fn scenario_with(files: std::collections::BTreeMap<String, String>, tick_hz: u16) -> App {
    let files = files
        .into_iter()
        .map(|(path, text)| (path, text.into_bytes()))
        .collect();
    let mut app = scenario(tick_hz);
    turts_pack::load_package(app.world_mut(), &archive_from_files(files))
        .expect("load configured economy fixture");
    app
}

pub fn advance(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

pub fn cid(text: &str) -> ContentId {
    ContentId::parse(text).expect("valid fixture content id")
}

pub fn economy_archive() -> turts_mod_runtime::archive::PackageArchive {
    archive_from_files(economy_files())
}

pub fn economy_files() -> std::collections::BTreeMap<String, Vec<u8>> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/economy");
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut std::collections::BTreeMap<String, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(dir).expect("read fixture dir") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    walk(&root, &root, &mut files);
    files
}

pub fn archive_from_files(
    files: std::collections::BTreeMap<String, Vec<u8>>,
) -> turts_mod_runtime::archive::PackageArchive {
    let bytes = turts_mod_runtime::pack::pack_files(&files).expect("pack fixture");
    turts_mod_runtime::archive::PackageArchive::parse(&bytes).expect("parse fixture")
}

pub trait AuditLookup {
    fn balance(&self, slot: u8, id: &str) -> &turts_sim::BalanceView;
}
impl AuditLookup for turts_sim::AuditState {
    fn balance(&self, slot: u8, id: &str) -> &turts_sim::BalanceView {
        let id = cid(id);
        let mut matches = self
            .economy
            .balances
            .iter()
            .filter(|balance| balance.slot == slot && balance.resource == id);
        let result = matches.next().expect("balance exists");
        assert!(matches.next().is_none(), "balance is unique");
        result
    }
}
