//! `turts-pack <dir>`: pack a package directory into a reproducible archive.
//!
//! Writes `<dir name>.tar` next to `<dir>` and prints `<file> <hash>`. Exits 2 on an error.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use turts_mod_api::ModError;
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::pack::pack_dir;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir] = args.as_slice() else {
        eprintln!("usage: turts-pack <dir>");
        return ExitCode::from(2);
    };
    match run(Path::new(dir)) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn run(dir: &Path) -> Result<String, ModError> {
    let bytes = pack_dir(dir)?;
    let file = archive_path(dir)?;
    fs::write(&file, &bytes).map_err(|error| {
        ModError::new(
            "",
            "pack",
            format!("cannot write `{}`: {error}", file.display()),
        )
    })?;
    Ok(format!("{} {}", file.display(), sha256_hex(&bytes)))
}

/// `<dir name>.tar` next to `dir`.
fn archive_path(dir: &Path) -> Result<PathBuf, ModError> {
    let error = |message: String| ModError::new("", "pack", message);
    // A path such as `.` has no name of its own; use the real path.
    let dir = if dir.file_name().is_some() {
        dir.to_path_buf()
    } else {
        dir.canonicalize()
            .map_err(|e| error(format!("cannot read `{}`: {e}", dir.display())))?
    };
    let name = dir
        .file_name()
        .ok_or_else(|| error(format!("cannot name an archive for `{}`", dir.display())))?;
    let mut file_name = name.to_os_string();
    file_name.push(".tar");
    Ok(dir.with_file_name(file_name))
}
