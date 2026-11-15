//! Host-only terrain entropy and replay provenance. No seed enters Booted or HTTP.
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use turts_mod_api::ModError;
use turts_sim::terrain::PrivateTerrainSeed;

use crate::config::HostConfig;

pub(super) struct PreparedPrivateTerrain {
    seed: PrivateTerrainSeed,
    path: PathBuf,
    keep: bool,
}
impl PreparedPrivateTerrain {
    pub(super) fn seed(&self) -> PrivateTerrainSeed {
        self.seed
    }
    pub(super) fn commit(mut self) {
        self.keep = true;
    }
}
impl Drop for PreparedPrivateTerrain {
    fn drop(&mut self) {
        if !self.keep {
            // Only this object's newly created file is eligible for cleanup.
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Resolve existing symlinks and normalize a not-yet-created file's suffix.
fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut normalized = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(part.as_os_str()),
        }
    }
    Ok(normalized)
}

fn resolved_path(path: &Path) -> std::io::Result<PathBuf> {
    let normalized = absolute_path(path)?;
    let mut existing = normalized.as_path();
    let mut suffix = Vec::new();
    while !existing.exists() {
        suffix.push(
            existing
                .file_name()
                .ok_or_else(|| std::io::Error::other("invalid private path"))?
                .to_os_string(),
        );
        existing = existing
            .parent()
            .ok_or_else(|| std::io::Error::other("invalid private path"))?;
    }
    let mut resolved = existing.canonicalize()?;
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

fn outside_http(path: &Path, served: Option<&Path>) -> std::io::Result<PathBuf> {
    let resolved = resolved_path(path)?;
    if let Some(served) = served {
        let root = resolved_path(served)?;
        if resolved.starts_with(&root) || absolute_path(path)?.starts_with(absolute_path(served)?) {
            return Err(std::io::Error::other(
                "private terrain files must stay outside static_dir",
            ));
        }
        let mut directories = vec![root];
        while let Some(directory) = directories.pop() {
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                if kind.is_symlink() {
                    if let Ok(target) = entry.path().canonicalize()
                        && resolved.starts_with(target)
                    {
                        return Err(std::io::Error::other(
                            "static_dir links to a private terrain path",
                        ));
                    }
                } else if kind.is_dir() {
                    directories.push(entry.path());
                }
            }
        }
    }
    Ok(resolved)
}

fn read_seed(path: &Path) -> std::io::Result<u64> {
    let mut bytes = Vec::new();
    File::open(path)?.take(65).read_to_end(&mut bytes)?;
    if bytes.len() > 64 {
        return Err(std::io::Error::other("private seed file exceeds 64 bytes"));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| std::io::Error::other("private seed must be unsigned decimal text"))?;
    text.trim().parse().map_err(|_| {
        std::io::Error::other("private seed must be an unsigned decimal 64-bit integer")
    })
}

pub(super) fn prepare(
    cfg: &HostConfig,
    package: &str,
    package_sha256: &str,
    enabled: bool,
) -> Result<Option<PreparedPrivateTerrain>, ModError> {
    let error = |field: &str, message: String| {
        ModError::new(package, field, message).with_file("server.ron")
    };
    if !enabled {
        if cfg.terrain_seed_file.is_some() || cfg.terrain_provenance.is_some() {
            return Err(error(
                "terrain_seed_file",
                "private terrain options require an authored terrain map".into(),
            ));
        }
        return Ok(None);
    }
    let served = cfg.static_dir.as_deref().map(Path::new);
    let seed = if let Some(path) = &cfg.terrain_seed_file {
        let path = outside_http(Path::new(path), served)
            .map_err(|e| error("terrain_seed_file", e.to_string()))?;
        read_seed(&path).map_err(|e| error("terrain_seed_file", e.to_string()))?
    } else {
        getrandom::u64().map_err(|e| {
            error(
                "terrain_seed_file",
                format!("cannot obtain operating-system entropy: {e}"),
            )
        })?
    };
    let path = if let Some(path) = &cfg.terrain_provenance {
        PathBuf::from(path)
    } else {
        let nonce = getrandom::u64().map_err(|e| {
            error(
                "terrain_provenance",
                format!("cannot obtain file nonce: {e}"),
            )
        })?;
        PathBuf::from(".turts-private").join(format!("terrain-{nonce:016x}.json"))
    };
    let path =
        outside_http(&path, served).map_err(|e| error("terrain_provenance", e.to_string()))?;
    let parent = path.parent().ok_or_else(|| {
        error(
            "terrain_provenance",
            "private file needs a parent directory".into(),
        )
    })?;
    let mut directory = fs::DirBuilder::new();
    directory.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory
        .create(parent)
        .map_err(|e| error("terrain_provenance", e.to_string()))?;
    let path =
        outside_http(&path, served).map_err(|e| error("terrain_provenance", e.to_string()))?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|e| error("terrain_provenance", e.to_string()))?;
    let prepared = PreparedPrivateTerrain {
        seed: PrivateTerrainSeed::new(seed),
        path,
        keep: false,
    };
    // A previously broken public link can become valid when this file is created.
    // Check again before writing any secret bytes.
    if let Err(e) = outside_http(&prepared.path, served) {
        drop(file);
        return Err(error("terrain_provenance", e.to_string()));
    }
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema":"turts-private-terrain-v1", "generator_version":1,
        "package_sha256":package_sha256, "seed":seed.to_string(),
    }))
    .map_err(|e| error("terrain_provenance", e.to_string()))?;
    let result = file.write_all(&bytes).and_then(|()| file.sync_all());
    drop(file);
    result.map_err(|e| error("terrain_provenance", e.to_string()))?;
    Ok(Some(prepared))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "turts-private-terrain-{:016x}",
                getrandom::u64().unwrap()
            ));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("public")).unwrap();
            Self(root)
        }
        fn config(&self) -> HostConfig {
            HostConfig {
                game_package: "game.tar".into(),
                http_listen: "127.0.0.1:0".into(),
                ws_listen: "127.0.0.1:0".into(),
                public_url: "http://127.0.0.1:7000".into(),
                tick_hz: 20,
                static_dir: Some(self.0.join("public").to_string_lossy().into_owned()),
                start_policy: None,
                bots: Vec::new(),
                bot_limits: Default::default(),
                terrain_seed_file: Some(self.0.join("seed.txt").to_string_lossy().into_owned()),
                terrain_provenance: Some(
                    self.0
                        .join("private/provenance.json")
                        .to_string_lossy()
                        .into_owned(),
                ),
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn provenance_is_private_replayable_non_overwriting_and_removed_on_failed_boot() {
        let fixture = Fixture::new();
        let cfg = fixture.config();
        let input = fixture.0.join("seed.txt");
        fs::write(&input, "18446744073709551615\n").unwrap();
        let prepared = prepare(&cfg, "quarry", "sha256:package", true)
            .unwrap()
            .unwrap();
        let path = prepared.path.clone();
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["seed"], "18446744073709551615");
        assert_eq!(value["generator_version"], 1);
        assert_eq!(value["package_sha256"], "sha256:package");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(prepared);
        assert!(!path.exists());
        assert_eq!(
            fs::read_to_string(&input).unwrap(),
            "18446744073709551615\n"
        );
        prepare(&cfg, "quarry", "sha256:package", true)
            .unwrap()
            .unwrap()
            .commit();
        let before = fs::read(&path).unwrap();
        assert!(prepare(&cfg, "quarry", "sha256:other", true).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[test]
    fn served_paths_and_seed_contents_never_become_provenance_or_diagnostics() {
        let fixture = Fixture::new();
        let mut cfg = fixture.config();
        let secret = "PRIVATE_INVALID_SEED_CONTENT";
        fs::write(fixture.0.join("seed.txt"), secret).unwrap();
        let error = match prepare(&cfg, "quarry", "hash", true) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("invalid seed accepted"),
        };
        assert!(!error.contains(secret));
        fs::write(fixture.0.join("public/seed.txt"), "42").unwrap();
        cfg.terrain_seed_file = Some(
            fixture
                .0
                .join("public/seed.txt")
                .to_string_lossy()
                .into_owned(),
        );
        assert!(prepare(&cfg, "quarry", "hash", true).is_err());
        assert!(!fixture.0.join("private/provenance.json").exists());
        cfg.terrain_seed_file = None;
        cfg.terrain_provenance = Some(
            fixture
                .0
                .join("public/../public/provenance.json")
                .to_string_lossy()
                .into_owned(),
        );
        assert!(prepare(&cfg, "quarry", "hash", true).is_err());
        assert!(!fixture.0.join("public/provenance.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn public_symlink_aliases_including_previously_broken_links_refuse_secret_writes() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let cfg = fixture.config();
        fs::write(fixture.0.join("seed.txt"), "42").unwrap();
        let public = fixture.0.join("public/seed-link");
        symlink(fixture.0.join("seed.txt"), &public).unwrap();
        assert!(prepare(&cfg, "quarry", "hash", true).is_err());
        fs::remove_file(public).unwrap();
        let output = fixture.0.join("private/provenance.json");
        symlink(&output, fixture.0.join("public/provenance-link")).unwrap();
        assert!(prepare(&cfg, "quarry", "hash", true).is_err());
        assert!(
            !output.exists(),
            "newly exposed empty file must be cleaned up"
        );
    }
}
