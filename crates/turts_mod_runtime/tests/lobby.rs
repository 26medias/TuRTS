//! Newly implemented start policies require an engine version that supports the handshake.
mod common;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

fn archive(policy: &str, api: &str) -> PackageArchive {
    let mut files = common::read_files(&common::package_dir("skirmish"));
    let manifest = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    let parsed: turts_mod_api::Manifest = ron::from_str(&manifest).unwrap();
    files.insert(
        "manifest.ron".into(),
        manifest.replace(&parsed.engine_api, api).into_bytes(),
    );
    let mut definition: turts_mod_api::MatchDef =
        ron::from_str(std::str::from_utf8(&files["match.ron"]).unwrap()).unwrap();
    definition.start = ron::from_str(policy).unwrap();
    files.insert(
        "match.ron".into(),
        ron::ser::to_string(&definition).unwrap().into_bytes(),
    );
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}

#[test]
fn ready_and_countdown_policies_have_an_explicit_version_floor() {
    for policy in [
        "AllSeated(countdown_secs:0)",
        "AllSeated(countdown_secs:3)",
        "AllSeatedReady(countdown_secs:3)",
    ] {
        archive(policy, "^0.2.9").content().unwrap();
        let result = archive(policy, "^0.2.8").content();
        assert!(result.is_err(), "{policy} was accepted for an older engine");
        let error = result.err().unwrap().to_string();
        assert!(
            error.contains("needs engine api 0.2.9"),
            "{policy}: {error}"
        );
    }
}

#[test]
fn immediate_packages_keep_their_existing_api_requirement() {
    archive("Immediate", "^0.2.8").content().unwrap();
}
