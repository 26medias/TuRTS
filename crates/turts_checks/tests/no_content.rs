//! No-content test (design 4.1): engine crates name no package, content id, or path under `mods/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PATTERN: &str = r#""(base|acme)[:"]|(base|acme)\.tar|rifleman|mods/"#;

/// Runs the design's grep, then drops hits whose text starts with a comment marker.
/// grep exit 1 means no hit. Any other non-zero exit is an error and panics.
fn scan(root: &Path) -> Vec<String> {
    let output = Command::new("grep")
        .args(["-rnE", PATTERN])
        .arg(root)
        .args(["--include=*.rs", "--exclude-dir=tests"])
        .output()
        .expect("grep starts");
    match output.status.code() {
        Some(0) | Some(1) => {}
        _ => panic!("grep failed: {}", String::from_utf8_lossy(&output.stderr)),
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| {
            // grep prints "path:line:text".
            let text = line.splitn(3, ':').nth(2).unwrap_or("").trim_start();
            !(text.starts_with("//") || text.starts_with("/*") || text.starts_with('*'))
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn no_content_in_engine_crates() {
    let crates_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let hits = scan(&crates_dir);
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn no_content_scan_goes_red_on_planted_file() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("no_content");
    let src = root.join("src");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("x.rs"),
        "fn f() {\n    let p = \"mods/base.tar\";\n}\n",
    )
    .unwrap();
    fs::write(src.join("y.rs"), "// rifleman\n").unwrap();
    let hits = scan(&root);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].contains("x.rs"), "{hits:?}");
}
