//! Reuse Stockyard bytes in memory. Only public RON authoring changes; no copied art files.
use std::{collections::BTreeMap, path::Path};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

pub fn renamed_package() -> PackageArchive {
    fn collect(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, files);
                continue;
            }
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            let bytes = std::fs::read(&path).unwrap();
            let bytes = if path.extension().is_some_and(|extension| extension == "ron") {
                let mut text = String::from_utf8(bytes)
                    .unwrap()
                    .replace("stockyard:", "visibility_stockpile:");
                if name == "manifest.ron" {
                    text = text
                        .replace("\"stockyard\"", "\"visibility_stockpile\"")
                        .replace("^0.2.15", "^0.2.16");
                }
                if name == "maps/field.ron" {
                    text = text.replacen('(', "(visibility: Some((cell_size: 2.0, sharing: Team, memory_limit: Some(128))), ", 1);
                }
                if name == "units/builder.ron" {
                    text = text.replacen('(', "(vision: Some((radius: 12.0)), ", 1);
                }
                text.into_bytes()
            } else {
                bytes
            };
            files.insert(name, bytes);
        }
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
    let mut files = BTreeMap::new();
    collect(&root, &root, &mut files);
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}
