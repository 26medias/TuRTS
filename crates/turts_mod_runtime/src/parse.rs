//! RON parse with structured errors.

use ron::error::{Error, SpannedError};
use serde::de::DeserializeOwned;
use turts_mod_api::ModError;

/// Parse a RON file into `T`. A failure becomes a `ModError` with `file` and `field_path`
/// set to `file`. The message never names a Rust type, because the `outer` field of the
/// ron error is not used. It ends with the line and column of the error.
pub fn parse_ron<T: DeserializeOwned>(
    package: &str,
    file: &str,
    text: &str,
) -> Result<T, ModError> {
    ron::from_str(text).map_err(|error| to_mod_error(package, file, error))
}

fn to_mod_error(package: &str, file: &str, error: SpannedError) -> ModError {
    let (message, allowed) = match error.code {
        Error::NoSuchStructField {
            expected, found, ..
        } => (format!("unknown field `{found}`"), Some(expected)),
        Error::MissingStructField { field, .. } => (format!("missing field `{field}`"), None),
        Error::NoSuchEnumVariant {
            expected, found, ..
        } => (format!("unknown variant `{found}`"), Some(expected)),
        Error::DuplicateStructField { field, .. } => (format!("duplicate field `{field}`"), None),
        other => (format!("cannot parse: {}", generic_message(&other)), None),
    };
    let message = format!(
        "{message} at line {} column {}",
        error.span.start.line, error.span.start.col
    );
    let error = ModError::new(package, file, message).with_file(file);
    match allowed {
        Some(names) => error.with_allowed(names.iter().map(|name| name.to_string()).collect()),
        None => error,
    }
}

/// A fixed message for every other ron error. ron's own text often names a Rust type, so it
/// never reaches a modder; the line and column added by the caller locate the problem.
fn generic_message(error: &Error) -> String {
    match error {
        Error::Eof => "unexpected end of file".to_string(),
        Error::ExpectedComma => "expected `,`".to_string(),
        Error::TrailingCharacters => "unexpected text after the value".to_string(),
        _ => "this value does not have the expected shape".to_string(),
    }
}
