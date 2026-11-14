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

#[test]
fn independent_review_ready_palette_rejects_invalid_colours() {
    let mut files = common::read_files(&common::package_dir("skirmish"));
    let manifest: turts_mod_api::Manifest =
        ron::from_str(std::str::from_utf8(&files["manifest.ron"]).unwrap()).unwrap();
    let manifest_text = std::str::from_utf8(&files["manifest.ron"])
        .unwrap()
        .replace(&manifest.engine_api, "^0.2.9");
    files.insert("manifest.ron".into(), manifest_text.into_bytes());
    let mut definition: turts_mod_api::MatchDef =
        ron::from_str(std::str::from_utf8(&files["match.ron"]).unwrap()).unwrap();
    definition.start = turts_mod_api::StartPolicy::AllSeatedReady { countdown_secs: 3 };
    files.insert(
        "match.ron".into(),
        ron::ser::to_string(&definition).unwrap().into_bytes(),
    );
    let interface_path = files
        .iter()
        .find(|(_, bytes)| {
            ron::from_str::<turts_mod_api::InterfaceDef>(std::str::from_utf8(bytes).unwrap_or(""))
                .is_ok()
        })
        .map(|(name, _)| name.clone())
        .unwrap();
    let mut interface: turts_mod_api::InterfaceDef =
        ron::from_str(std::str::from_utf8(&files[&interface_path]).unwrap()).unwrap();
    interface.slot_colours = vec!["invalid-red".into(), "invalid-blue".into()];
    files.insert(
        interface_path,
        ron::ser::to_string(&interface).unwrap().into_bytes(),
    );
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert!(
        archive.content().is_err(),
        "a lobby package must reject invalid palette values instead of displaying both seats as white"
    );
}

#[test]
fn host_override_uses_the_same_strict_palette_contract() {
    use turts_mod_runtime::content::validate_lobby_palette;
    for colours in [vec!["#0072B2", "#E69F00"], vec!["#abcdef", "#123456"]] {
        assert!(
            validate_lobby_palette(&colours.into_iter().map(String::from).collect::<Vec<_>>())
                .is_ok()
        );
    }
    for colours in [
        vec![],
        vec!["#0072B2"],
        vec!["bad", "#E69F00"],
        vec!["#abcdef", "#ABCDEF"],
        vec!["#abcdef00", "#123456"],
        vec!["#12345g", "#123456"],
    ] {
        assert!(
            validate_lobby_palette(&colours.into_iter().map(String::from).collect::<Vec<_>>())
                .is_err()
        );
    }
    assert!(validate_lobby_palette(&vec!["#123456".into(); 256]).is_err());
}
