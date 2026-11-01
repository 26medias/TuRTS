//! Package acquisition: the checks between the bytes on the wire and the data the client uses.
//!
//! These are pure functions. The phase machine of the client calls them in this order:
//! `check_advertisement`, `check_lockfile`, `packages_to_fetch`, `accept_package` for each
//! package, `build_acquired` (design 4.4 and 5).

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::content::ClientContent;
use turts_mod_api::{AssetsDef, InterfaceDef};
use turts_mod_api::{ContentId, ENGINE_API_VERSION, ExecutionContext, LockEntry, Lockfile, Shape};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::hash::verify_hash;
use turts_mod_runtime::lock::read_lockfile;
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION};

/// What the client keeps after it has accepted the server's packages.
#[derive(Clone, Debug, PartialEq)]
pub struct Acquired {
    /// The hash that `Hello` carries.
    pub lockfile_hash: String,
    /// The visual shape of each unit, by full content id.
    pub visuals: HashMap<String, Shape>,
    /// The size of the map on the ground plane.
    pub map_size: (f32, f32),
    /// The display name of each unit, by full content id.
    pub unit_names: HashMap<String, String>,
    pub content: ClientContent,
    pub presentation: PresentationContent,
}
/// Authored interface and its accepted package-local image/font payloads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresentationContent {
    pub interface: Option<InterfaceDef>,
    pub interface_package: Option<String>,
    pub images: BTreeMap<String, PresentationImage>,
    pub fonts: BTreeMap<String, Vec<u8>>,
    pub cursor_assets: BTreeMap<String, (String, (u16, u16))>,
    pub cursors: BTreeMap<String, (String, (u16, u16))>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresentationImage {
    pub bytes: Vec<u8>,
    pub nine_slice: Option<(u16, u16, u16, u16)>,
}

/// Read the advertisement and check that the client can talk to this server: the same wire
/// protocol version and the same engine API version.
pub fn check_advertisement(bytes: &[u8]) -> Result<Advertisement, String> {
    let ad: Advertisement =
        serde_json::from_slice(bytes).map_err(|error| format!("advertisement: {error}"))?;
    if ad.protocol_version != NET_PROTOCOL_VERSION {
        return Err(format!(
            "advertisement: the server speaks protocol version {} but this client speaks {NET_PROTOCOL_VERSION}",
            ad.protocol_version
        ));
    }
    if ad.engine_version != ENGINE_API_VERSION {
        return Err(format!(
            "advertisement: the server runs engine version {} but this client runs {ENGINE_API_VERSION}",
            ad.engine_version
        ));
    }
    Ok(ad)
}

/// Read the lockfile. The bytes are checked against the advertised hash first, and are not
/// parsed when the hash differs (design 5).
pub fn check_lockfile(
    bytes: &[u8],
    advertised_hash: &str,
    advertised_mod_id: &str,
) -> Result<Lockfile, String> {
    verify_hash(bytes, advertised_hash)
        .map_err(|error| format!("lockfile hash mismatch: {}", error.message))?;
    let lockfile = read_lockfile(bytes).map_err(|error| error.to_string())?;
    let mut seen = HashSet::new();
    for entry in &lockfile.packages {
        if !seen.insert(entry.id.as_str()) {
            return Err(format!("lockfile: package `{}` has two entries", entry.id));
        }
    }
    if !seen.contains(advertised_mod_id) {
        return Err(format!(
            "lockfile: the advertised package `{advertised_mod_id}` has no entry"
        ));
    }
    Ok(lockfile)
}

/// The packages the client downloads: context `Client` or `Shared`. A `Server` package is never
/// requested (spec 6.5).
pub fn packages_to_fetch(lock: &Lockfile) -> Vec<&LockEntry> {
    lock.packages
        .iter()
        .filter(|entry| entry.context != ExecutionContext::Server)
        .collect()
}

/// Accept downloaded package bytes. The bytes are checked against the lock entry hash before
/// they are parsed. Then the parsed package must match the rest of the entry.
pub fn accept_package(bytes: &[u8], entry: &LockEntry) -> Result<PackageArchive, String> {
    verify_hash(bytes, &entry.hash).map_err(|mut error| {
        error.package = entry.id.clone();
        error.to_string()
    })?;
    let archive = PackageArchive::parse(bytes).map_err(|error| error.to_string())?;
    archive
        .check_against_lock(entry)
        .map_err(|error| error.to_string())?;
    Ok(archive)
}

/// Build what the client keeps from the accepted packages. Every unit has exactly one visual,
/// and every visual names a defined unit, else this is an error that names the file and the
/// field (design 4.4; `turts_mod_runtime::visuals::visuals_by_content`). The map is the first map
/// of the first package that has one.
pub fn build_acquired(
    archives: &[PackageArchive],
    lockfile_hash: String,
) -> Result<Acquired, String> {
    let content = ClientContent::from_archives(archives).map_err(|error| error.to_string())?;
    let presentation = presentation_from_archives(archives)?;
    let visuals = content.shapes.clone();
    let mut unit_names = HashMap::new();
    let mut map_size = None;
    for archive in archives {
        let package = archive.id();
        for (_, unit) in archive.units().map_err(|error| error.to_string())? {
            unit_names.insert(ContentId::new(&package, &unit.id).to_string(), unit.name);
        }
        if map_size.is_none() {
            let maps = archive.maps().map_err(|error| error.to_string())?;
            map_size = maps.first().map(|(_, map)| (map.size.x, map.size.y));
        }
    }
    let map_size = map_size.ok_or_else(|| "the packages have no map".to_string())?;
    Ok(Acquired {
        lockfile_hash,
        visuals,
        map_size,
        unit_names,
        content,
        presentation,
    })
}

fn presentation_from_archives(archives: &[PackageArchive]) -> Result<PresentationContent, String> {
    let mut presentation = PresentationContent::default();
    for archive in archives {
        if presentation.interface.is_none()
            && let Some((_, interface)) = archive
                .interfaces()
                .map_err(|error| error.to_string())?
                .into_iter()
                .next()
        {
            presentation.interface = Some(interface);
            presentation.interface_package = Some(archive.id().0);
        }
        for (_, assets) in archive.assets().map_err(|error| error.to_string())? {
            merge_asset_bytes(archive, assets, &mut presentation)?;
        }
    }
    if let Some(interface) = presentation.interface.as_ref() {
        let interface_package = presentation
            .interface_package
            .as_deref()
            .expect("selected interface must retain its archive owner");
        presentation.cursors = interface
            .cursors
            .iter()
            .filter_map(|(context, asset)| {
                let local_cursor_key = qualified_asset_id(interface_package, asset);
                let cursor = presentation.cursor_assets.get(&local_cursor_key).cloned();
                cursor.map(|value| (context.clone(), value))
            })
            .collect();
    }
    Ok(presentation)
}

fn merge_asset_bytes(
    archive: &PackageArchive,
    assets: AssetsDef,
    presentation: &mut PresentationContent,
) -> Result<(), String> {
    for image in assets.images {
        let bytes = archive
            .file(&image.path)
            .ok_or_else(|| format!("{}: missing image asset {}", archive.id().0, image.path))?;
        presentation.images.insert(
            qualified_asset_id(&archive.id().0, &image.id),
            PresentationImage {
                bytes: bytes.to_vec(),
                nine_slice: image.nine_slice,
            },
        );
    }
    for font in assets.fonts {
        let bytes = archive
            .file(&font.path)
            .ok_or_else(|| format!("{}: missing font asset {}", archive.id().0, font.path))?;
        presentation.fonts.insert(
            qualified_asset_id(&archive.id().0, &font.id),
            bytes.to_vec(),
        );
    }
    for cursor in assets.cursors {
        presentation.cursor_assets.insert(
            qualified_asset_id(&archive.id().0, &cursor.id),
            (
                qualified_asset_id(&archive.id().0, &cursor.image),
                cursor.hotspot,
            ),
        );
    }
    Ok(())
}

fn qualified_asset_id(package: &str, local_id: &str) -> String {
    format!("{package}:{local_id}")
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use turts_mod_runtime::pack::pack_dir;

    fn copy_tree(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&source_path, &destination_path);
            } else {
                fs::copy(source_path, destination_path).unwrap();
            }
        }
    }

    fn rewrite_package_refs(directory: &Path) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                rewrite_package_refs(&path);
            } else if path.extension().is_some_and(|extension| extension == "ron") {
                let source = fs::read_to_string(&path).unwrap();
                fs::write(path, source.replace("muster:", "other:")).unwrap();
            }
        }
    }

    #[test]
    fn interface_assets_keep_their_package_provenance_when_local_ids_collide() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture = workspace.join("crates/turts_pack/tests/fixtures/muster_hud1");
        let alternate_image = workspace.join("mods/muster/assets/images/cursor_move.png");
        let alternate_font = workspace.join("mods/muster/assets/fonts/Roboto-Regular.ttf");
        let temp = std::env::temp_dir().join(format!(
            "turts-presentation-collision-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let first_dir = temp.join("muster");
        let second_dir = temp.join("other");
        copy_tree(&fixture, &first_dir);
        copy_tree(&fixture, &second_dir);

        let manifest_path = second_dir.join("manifest.ron");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        fs::write(
            &manifest_path,
            manifest.replacen("id: \"muster\"", "id: \"other\"", 1),
        )
        .unwrap();
        rewrite_package_refs(&second_dir);

        // Keep the same local names but make the dependency payloads observably different.
        let mut other_font = fs::read(alternate_font).unwrap();
        other_font.extend_from_slice(b"other-package-font-padding");
        fs::write(
            second_dir.join("assets/fonts/Roboto-Regular.ttf"),
            other_font,
        )
        .unwrap();
        fs::copy(
            &alternate_image,
            second_dir.join("assets/images/icon_attack.png"),
        )
        .unwrap();
        fs::copy(
            &alternate_image,
            second_dir.join("assets/images/cursor_attack.png"),
        )
        .unwrap();

        for (directory, nine_slice, hotspot) in [
            (&first_dir, "Some((1, 2, 3, 4))", "(12, 12)"),
            (&second_dir, "Some((5, 6, 7, 8))", "(3, 4)"),
        ] {
            let assets_path = directory.join("assets/assets.ron");
            let mut assets = fs::read_to_string(&assets_path).unwrap();
            assets = assets.replacen(
                "(id: \"muster_icon_attack\", path: \"assets/images/icon_attack.png\", nine_slice: None)",
                &format!("(id: \"muster_icon_attack\", path: \"assets/images/icon_attack.png\", nine_slice: {nine_slice})"),
                1,
            );
            if hotspot != "(12, 12)" {
                assets = assets.replace("hotspot: (12, 12)", &format!("hotspot: {hotspot}"));
            }
            fs::write(assets_path, assets).unwrap();
        }

        let first = PackageArchive::parse(&pack_dir(&first_dir).unwrap()).unwrap();
        let second = PackageArchive::parse(&pack_dir(&second_dir).unwrap()).unwrap();
        let acquired = build_acquired(&[first, second], String::new())
            .expect("both namespace-correct packages are valid accepted content");
        let presentation = acquired.presentation;

        assert_eq!(presentation.interface_package.as_deref(), Some("muster"));
        assert_eq!(
            presentation.images["muster:muster_icon_attack"].nine_slice,
            Some((1, 2, 3, 4))
        );
        assert_eq!(
            presentation.images["other:muster_icon_attack"].nine_slice,
            Some((5, 6, 7, 8))
        );
        assert_ne!(
            presentation.images["muster:muster_icon_attack"].bytes,
            presentation.images["other:muster_icon_attack"].bytes
        );
        assert_ne!(
            presentation.fonts["muster:muster_ui"],
            presentation.fonts["other:muster_ui"]
        );
        assert_eq!(
            presentation.cursors["attack"].0,
            "muster:muster_cursor_attack"
        );
        assert_eq!(presentation.cursors["attack"].1, (12, 12));
        assert_eq!(
            presentation.cursor_assets["other:muster_cursor_attack"].1,
            (3, 4)
        );
        assert_ne!(
            presentation.cursor_assets["muster:muster_cursor_attack"].0,
            presentation.cursor_assets["other:muster_cursor_attack"].0
        );

        let mut decoded_images = bevy::asset::Assets::<bevy::prelude::Image>::default();
        let mut decoded_fonts = bevy::asset::Assets::<bevy::prelude::Font>::default();
        let decoded = crate::widgets::assets::load_presentation_assets(
            &presentation,
            &mut decoded_images,
            &mut decoded_fonts,
            None,
        )
        .expect("both accepted archives also decode as renderer assets");
        let selected_pixels = decoded_images
            .get(decoded.image("muster:muster_icon_attack").unwrap().id())
            .unwrap()
            .data
            .as_ref()
            .unwrap();
        let dependency_pixels = decoded_images
            .get(decoded.image("other:muster_icon_attack").unwrap().id())
            .unwrap()
            .data
            .as_ref()
            .unwrap();
        assert_ne!(selected_pixels, dependency_pixels);
        assert_ne!(
            decoded_images
                .get(
                    decoded
                        .cursor_image_for("muster:muster_cursor_attack", (12, 12))
                        .unwrap()
                        .id()
                )
                .unwrap()
                .data,
            decoded_images
                .get(
                    decoded
                        .cursor_image_for("other:muster_cursor_attack", (3, 4))
                        .unwrap()
                        .id()
                )
                .unwrap()
                .data
        );
        assert_ne!(
            decoded_fonts
                .get(decoded.font("muster:muster_ui").unwrap().id())
                .unwrap()
                .data,
            decoded_fonts
                .get(decoded.font("other:muster_ui").unwrap().id())
                .unwrap()
                .data,
        );

        fs::remove_dir_all(temp).unwrap();
    }
}
