//! Portable API16 sight validation. This does not install a simulation policy.
use crate::content::{check_finite_positive, error};
use turts_mod_api::{
    MAX_REMEMBERED_SUBJECTS, MAX_VISIBILITY_CELLS, MAX_VISIBILITY_COVERAGE_BYTES, MapDef, ModError,
    PackageId, SLOT_COUNT, VisibilityGrid,
};

/// Validate an authored map before a caller stages world installation.
/// AllVisible maps need no coverage grid or new allocation.
pub fn validate_visibility(
    origin: &PackageId,
    file: &str,
    map: &MapDef,
) -> Result<Option<VisibilityGrid>, ModError> {
    let Some(definition) = &map.visibility else {
        return Ok(None);
    };
    check_finite_positive(
        origin,
        file,
        "visibility.cell_size",
        definition.cell_size,
        false,
    )?;
    check_finite_positive(origin, file, "size.x", map.size.x, false)?;
    check_finite_positive(origin, file, "size.y", map.size.y, false)?;
    if !(1..=MAX_REMEMBERED_SUBJECTS).contains(&definition.effective_memory_limit()) {
        return Err(error(
            origin,
            file,
            "visibility.memory_limit",
            "must be within 1..=65536",
        ));
    }
    let dimension = |value: f32| -> Result<u32, ModError> {
        let cells = (f64::from(value) / f64::from(definition.cell_size)).ceil();
        if !cells.is_finite() || cells < 1.0 || cells > f64::from(u32::MAX) {
            return Err(error(
                origin,
                file,
                "visibility.cell_size",
                "coverage grid dimension exceeds u32",
            ));
        }
        Ok(cells as u32)
    };
    let columns = dimension(map.size.x)?;
    let rows = dimension(map.size.y)?;
    let cells = u64::from(columns)
        .checked_mul(u64::from(rows))
        .filter(|cells| *cells <= MAX_VISIBILITY_CELLS)
        .ok_or_else(|| {
            error(
                origin,
                file,
                "visibility.cell_size",
                "coverage grid exceeds 1048576 cells",
            )
        })?;
    // Reserve eight masks per slot for start/tactical/completed current/explored
    // bases and staging. The simulation must stay within this validated bound.
    let bytes = cells
        .checked_add(7)
        .map(|cells| cells / 8)
        .and_then(|bytes| bytes.checked_mul(8))
        .and_then(|bytes| bytes.checked_mul(u64::from(SLOT_COUNT)))
        .filter(|bytes| *bytes <= MAX_VISIBILITY_COVERAGE_BYTES)
        .ok_or_else(|| {
            error(
                origin,
                file,
                "visibility.cell_size",
                "slot coverage storage exceeds 64 MiB",
            )
        })?;
    let _ = bytes;
    Ok(Some(VisibilityGrid {
        size: turts_mod_api::WorldPos {
            x: map.size.x,
            y: map.size.y,
        },
        cell_size: definition.cell_size,
        columns,
        rows,
    }))
}
