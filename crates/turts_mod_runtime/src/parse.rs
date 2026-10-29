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
    let (message, allowed, hint) = match error.code {
        Error::NoSuchStructField {
            expected, found, ..
        } => {
            let suggestion = expected
                .iter()
                .enumerate()
                .map(|(index, name)| (edit_distance(&found, name), index, *name))
                .filter(|(distance, _, _)| *distance <= 2)
                .min_by_key(|(distance, index, _)| (*distance, *index))
                .map(|(_, _, name)| format!("; did you mean `{name}`?"));
            (
                format!("unknown field `{found}`"),
                Some(expected),
                suggestion,
            )
        }
        Error::MissingStructField { field, .. } => (format!("missing field `{field}`"), None, None),
        Error::NoSuchEnumVariant {
            expected, found, ..
        } => (format!("unknown variant `{found}`"), Some(expected), None),
        Error::DuplicateStructField { field, .. } => {
            (format!("duplicate field `{field}`"), None, None)
        }
        other => (
            format!("cannot parse: {}", generic_message(&other)),
            None,
            None,
        ),
    };
    let message = format!(
        "{message} at line {} column {}{}",
        error.span.start.line,
        error.span.start.col,
        hint.unwrap_or_default()
    );
    let error = ModError::new(package, file, message).with_file(file);
    match allowed {
        Some(names) => error.with_allowed(names.iter().map(|name| name.to_string()).collect()),
        None => error,
    }
}

fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut distances: Vec<usize> = (0..=right.len()).collect();

    for (left_index, left_char) in left.iter().enumerate() {
        let mut diagonal = distances[0];
        distances[0] = left_index + 1;
        for (right_index, right_char) in right.iter().enumerate() {
            let above = distances[right_index + 1];
            let substitution = if left_char == right_char { 0 } else { 1 };
            distances[right_index + 1] = (diagonal + substitution)
                .min(distances[right_index] + 1)
                .min(above + 1);
            diagonal = above;
        }
    }

    distances[right.len()]
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
