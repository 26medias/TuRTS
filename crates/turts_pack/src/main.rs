//! `turts-pack <dir>`: check a package directory and pack it into a reproducible archive.
//!
//! Writes `<dir name>.tar` next to `<dir>` and prints `<file> <hash>`. Exits 2 on an error and
//! then writes no archive. `turts-pack --help` prints the usage.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use turts_mod_api::ModError;
use turts_mod_runtime::{
    archive::PackageArchive, content_mc_validation::layout_warnings, hash::sha256_hex,
};
use turts_pack::pack_checked;

const USAGE: &str = "\
usage: turts-pack <dir>
       turts-pack --help

Pack the package directory <dir> into the archive <dir>.tar, next to <dir>.
Print the archive path and its sha256 hash on one line. The archive is reproducible:
the same files give the same bytes.

Before it writes the archive, turts-pack checks the package the way a server and a
client check it:
  - The manifest and the exported files parse. An unknown field is an error.
  - The manifest requests only known capabilities, with no duplicates.
  - Libraries export no maps. Games export exactly one map. Only libraries can be dependencies.
  - The units and the map follow the server rules: speed and radius in range, owner
    slot is a real slot, positions inside the map, exactly one map. An error message
    gives the limit that the value broke.
  - Every unit has exactly one visual. Every visual names a unit of the package.

On the first problem, turts-pack prints the package, the file, and the field to stderr,
writes no archive, and exits with 2. On success it exits with 0.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [option] if option.starts_with('-') => {
            eprintln!("turts-pack: unknown option `{option}`\n\n{USAGE}");
            ExitCode::from(2)
        }
        [dir] => match run(Path::new(dir)) {
            Ok((line, warnings)) => {
                for warning in warnings {
                    eprintln!("{warning}");
                }
                println!("{line}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(dir: &Path) -> Result<(String, Vec<String>), ModError> {
    let bytes = pack_checked(dir)?;
    let archive = PackageArchive::parse(&bytes)?;
    let warnings = layout_warnings(&archive)?;
    let file = archive_path(dir)?;
    fs::write(&file, &bytes).map_err(|error| {
        ModError::new(
            "",
            "pack",
            format!("cannot write `{}`: {error}", file.display()),
        )
    })?;
    Ok((
        format!("{} {}", file.display(), sha256_hex(&bytes)),
        warnings,
    ))
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
