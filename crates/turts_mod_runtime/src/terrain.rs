//! API17 public authoring validation. No private seed or realized columns are loaded here.
use crate::content::{checked_content_ref, error, local_id};
use std::collections::BTreeSet;
use turts_mod_api::*;
use turts_rules::terrain::{geometry, material};

fn ratio(origin: &PackageId, file: &str, field: &str, value: Ratio) -> Result<(), ModError> {
    if value.denominator == 0 {
        return Err(error(origin, file, field, "denominator must be positive"));
    }
    Ok(())
}
fn pure_ratio(value: Ratio) -> turts_rules::terrain::Ratio {
    turts_rules::terrain::Ratio {
        numerator: u64::from(value.numerator),
        denominator: u64::from(value.denominator),
    }
}
fn source<'a>(catalog: &'a ContentCatalog, id: &ContentId) -> &'a str {
    catalog
        .source_files
        .get(id)
        .map_or("content", String::as_str)
}
pub(crate) fn insert_source(
    origin: &PackageId,
    file: &str,
    local: &str,
    catalog: &mut ContentCatalog,
) -> Result<ContentId, ModError> {
    let id = local_id(origin, file, local)?;
    if catalog.source_files.contains_key(&id) {
        return Err(error(
            origin,
            file,
            "id",
            "duplicate definition id across content kinds",
        ));
    }
    catalog.source_files.insert(id.clone(), file.into());
    Ok(id)
}
fn rates(
    origin: &PackageId,
    file: &str,
    field: &str,
    rows: Vec<TerrainResourceRate>,
) -> Result<Vec<ResolvedTerrainResourceRate>, ModError> {
    let mut seen = BTreeSet::new();
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            let path = format!("{field}[{i}]");
            ratio(
                origin,
                file,
                &format!("{path}.per_world_cube"),
                row.per_world_cube,
            )?;
            if row.per_world_cube.numerator == 0 {
                return Err(error(
                    origin,
                    file,
                    &path,
                    "resource coefficient must be positive; omit zero rows",
                ));
            }
            let resource =
                checked_content_ref(origin, file, &format!("{path}.resource"), &row.resource)?;
            if !seen.insert(resource.clone()) {
                return Err(error(origin, file, &path, "duplicate resource"));
            }
            Ok(ResolvedTerrainResourceRate {
                resource,
                per_world_cube: row.per_world_cube,
            })
        })
        .collect()
}
/// Populate public typed records after ordinary content resolution. Foreign links
/// may remain deferred only to a declared dependency; final aggregate validation
/// is mandatory before installation.
pub fn install_terrain(
    manifest: &Manifest,
    catalog: &mut ContentCatalog,
    materials: Vec<(String, TerrainMaterialDef)>,
    classes: Vec<(String, TerrainMovementClassDef)>,
    tools: Vec<(String, TerrainToolDef)>,
) -> Result<(), ModError> {
    let origin = PackageId::parse(&manifest.id).map_err(|message| {
        error(
            &PackageId(manifest.id.clone()),
            "manifest.ron",
            "id",
            message,
        )
    })?;
    for (file, value) in materials {
        let id = insert_source(&origin, &file, &value.id, catalog)?;
        let color = value.surface_color.as_bytes();
        if color.len() != 7 || color[0] != b'#' || !color[1..].iter().all(u8::is_ascii_hexdigit) {
            return Err(error(&origin, &file, "surface_color", "must be #RRGGBB"));
        }
        ratio(
            &origin,
            &file,
            "work_per_world_cube",
            value.work_per_world_cube,
        )?;
        if value.work_per_world_cube.numerator == 0 {
            return Err(error(
                &origin,
                &file,
                "work_per_world_cube",
                "required work must be positive",
            ));
        }
        catalog.terrain_materials.insert(
            id.clone(),
            ResolvedTerrainMaterial {
                id,
                name: value.name,
                surface_color: value.surface_color,
                hardness: value.hardness,
                work_per_world_cube: value.work_per_world_cube,
                natural_outputs: rates(&origin, &file, "natural_outputs", value.natural_outputs)?,
                fill_inputs: rates(&origin, &file, "fill_inputs", value.fill_inputs)?,
                placed_removal: value.placed_removal,
            },
        );
    }
    for (file, value) in classes {
        let id = insert_source(&origin, &file, &value.id, catalog)?;
        ratio(&origin, &file, "max_grade", value.max_grade)?;
        catalog.terrain_movement_classes.insert(
            id.clone(),
            ResolvedTerrainMovementClass {
                id,
                name: value.name,
                max_step: value.max_step,
                max_grade: value.max_grade,
            },
        );
    }
    for (file, value) in tools {
        let id = insert_source(&origin, &file, &value.id, catalog)?;
        if value.work_per_second == 0 {
            return Err(error(&origin, &file, "work_per_second", "must be positive"));
        }
        if !value.reach.is_finite() || value.reach < 0.0 {
            return Err(error(
                &origin,
                &file,
                "reach",
                "must be finite and nonnegative",
            ));
        }
        if value.allowed_materials.is_empty() {
            return Err(error(
                &origin,
                &file,
                "allowed_materials",
                "must not be empty",
            ));
        }
        let allowed_materials = value
            .allowed_materials
            .iter()
            .enumerate()
            .map(|(i, text)| {
                checked_content_ref(&origin, &file, &format!("allowed_materials[{i}]"), text)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if allowed_materials.iter().collect::<BTreeSet<_>>().len() != allowed_materials.len() {
            return Err(error(
                &origin,
                &file,
                "allowed_materials",
                "duplicate material",
            ));
        }
        let operation = match value.operation {
            TerrainOperation::Excavate => ResolvedTerrainOperation::Excavate,
            TerrainOperation::Fill { material } => ResolvedTerrainOperation::Fill {
                material: checked_content_ref(&origin, &file, "operation.material", &material)?,
            },
        };
        catalog.terrain_tools.insert(
            id.clone(),
            ResolvedTerrainTool {
                id,
                name: value.name,
                operation,
                work_per_second: value.work_per_second,
                reach: value.reach,
                allowed_materials,
                max_hardness: value.max_hardness,
                output_destination: value.output_destination,
            },
        );
    }
    validate_links(&origin, catalog, Some(manifest))
}
fn require<T>(
    origin: &PackageId,
    file: &str,
    field: &str,
    id: &ContentId,
    entries: &std::collections::BTreeMap<ContentId, T>,
    partial: Option<&Manifest>,
) -> Result<(), ModError> {
    if let Some(manifest) = partial
        && id.package != *origin
    {
        if manifest.dependencies.0.contains_key(&id.package.0) {
            return Ok(());
        }
        return Err(error(
            origin,
            file,
            field,
            "reference is not in the package or its declared dependencies",
        ));
    }
    if !entries.contains_key(id) {
        return Err(error(
            origin,
            file,
            field,
            format!("unknown terrain reference `{id}`"),
        ));
    }
    Ok(())
}
fn validate_links(
    origin: &PackageId,
    catalog: &ContentCatalog,
    partial: Option<&Manifest>,
) -> Result<(), ModError> {
    for material in catalog.terrain_materials.values() {
        let file = source(catalog, &material.id);
        for (field, rows) in [
            ("natural_outputs", &material.natural_outputs),
            ("fill_inputs", &material.fill_inputs),
        ] {
            for row in rows {
                require(
                    origin,
                    file,
                    field,
                    &row.resource,
                    &catalog.resources,
                    partial,
                )?;
                if let Some(resource) = catalog.resources.get(&row.resource)
                    && !matches!(resource.kind, ResourceKind::Item | ResourceKind::Balance)
                {
                    return Err(error(
                        origin,
                        file,
                        field,
                        "terrain resources must be Item or Balance, never Supply",
                    ));
                }
            }
        }
    }
    for tool in catalog.terrain_tools.values() {
        let file = source(catalog, &tool.id);
        let mut used = tool.allowed_materials.clone();
        if let ResolvedTerrainOperation::Fill { material } = &tool.operation {
            used.push(material.clone());
        }
        for id in &used {
            require(
                origin,
                file,
                "allowed_materials",
                id,
                &catalog.terrain_materials,
                partial,
            )?;
            if let Some(material) = catalog.terrain_materials.get(id) {
                // Hardness limits apply to exposed work material, not merely the fill input identity.
                if tool.allowed_materials.contains(id) && material.hardness > tool.max_hardness {
                    return Err(error(
                        origin,
                        file,
                        "max_hardness",
                        format!("cannot work declared material `{id}`"),
                    ));
                }
                for row in material
                    .natural_outputs
                    .iter()
                    .filter(|_| matches!(tool.operation, ResolvedTerrainOperation::Excavate))
                {
                    if let Some(resource) = catalog.resources.get(&row.resource) {
                        let required = match tool.output_destination {
                            TerrainOutputDestination::Inventory => ResourceKind::Item,
                            TerrainOutputDestination::SlotBalances => ResourceKind::Balance,
                        };
                        if resource.kind != required {
                            return Err(error(
                                origin,
                                file,
                                "output_destination",
                                "material resource kind does not match the tool endpoint",
                            ));
                        }
                    }
                }
            }
        }
    }
    for unit in catalog.units.values() {
        let file = source(catalog, &unit.id);
        if let Some(class) = &unit.terrain_class {
            require(
                origin,
                file,
                "terrain_class",
                class,
                &catalog.terrain_movement_classes,
                partial,
            )?;
        }
        if let Some(cargo) = unit.terrain_cargo
            && (cargo.unload_per_second == 0
                || !cargo.interaction_range.is_finite()
                || cargo.interaction_range < 0.0
                || unit.inventory.is_none())
        {
            return Err(error(
                origin,
                file,
                "terrain_cargo",
                "requires an inventory, positive unload rate and finite nonnegative reach",
            ));
        }
        let mut seen = BTreeSet::new();
        for id in &unit.terrain_tools {
            if !seen.insert(id) {
                return Err(error(origin, file, "terrain_tools", "duplicate tool"));
            }
            require(
                origin,
                file,
                "terrain_tools",
                id,
                &catalog.terrain_tools,
                partial,
            )?;
            if let Some(tool) = catalog.terrain_tools.get(id)
                && let ResolvedTerrainOperation::Fill { material } = &tool.operation
                && let Some(material) = catalog.terrain_materials.get(material)
            {
                for row in &material.fill_inputs {
                    if catalog
                        .resources
                        .get(&row.resource)
                        .is_some_and(|r| r.kind == ResourceKind::Item)
                        && !unit
                            .inventory
                            .as_ref()
                            .is_some_and(|inventory| inventory.accepts.contains(&row.resource))
                    {
                        return Err(error(
                            origin,
                            file,
                            "inventory.accepts",
                            "fill Item inputs require a compatible actor inventory",
                        ));
                    }
                }
            }
            if let Some(tool) = catalog.terrain_tools.get(id)
                && tool.output_destination == TerrainOutputDestination::Inventory
            {
                let Some(inventory) = &unit.inventory else {
                    return Err(error(
                        origin,
                        file,
                        "terrain_tools",
                        "inventory tool requires a physical inventory",
                    ));
                };
                if unit.terrain_cargo.is_none() {
                    return Err(error(
                        origin,
                        file,
                        "terrain_cargo",
                        "inventory tool requires explicit timed terrain cargo unloading",
                    ));
                }
                let outputs = tool
                    .allowed_materials
                    .iter()
                    .filter_map(|id| catalog.terrain_materials.get(id))
                    .flat_map(|material| &material.natural_outputs);
                for row in outputs {
                    if !inventory.accepts.contains(&row.resource) {
                        return Err(error(
                            origin,
                            file,
                            "inventory.accepts",
                            "inventory does not accept a tool's natural output",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
/// Required on a complete merged catalog before any terrain-enabled installation.
pub fn validate_terrain_catalog(
    origin: &PackageId,
    catalog: &ContentCatalog,
) -> Result<(), ModError> {
    validate_links(origin, catalog, None)
}

fn rectangle(
    origin: &PackageId,
    file: &str,
    field: &str,
    rect: TerrainRect,
    columns: u32,
    rows: u32,
) -> Result<u64, ModError> {
    if rect.min.x >= rect.max.x
        || rect.min.y >= rect.max.y
        || rect.max.x > columns
        || rect.max.y > rows
    {
        return Err(error(
            origin,
            file,
            field,
            "requires a nonempty in-grid half-open rectangle",
        ));
    }
    Ok(u64::from(rect.max.x - rect.min.x) * u64::from(rect.max.y - rect.min.y))
}
/// Expand only public initial geometry with checked, bounded allocation.
pub fn initial_elevations(
    origin: &PackageId,
    file: &str,
    map: &MapDef,
) -> Result<Option<Vec<i32>>, ModError> {
    let Some(t) = &map.terrain else {
        return Ok(None);
    };
    let cells = u64::from(t.columns) * u64::from(t.rows);
    material::storage_bound(cells, t.floor, t.ceiling).map_err(|e| {
        error(
            origin,
            file,
            "terrain",
            format!("invalid public cell/height/storage bounds: {e:?}"),
        )
    })?;
    if t.length_scale == 0 || t.cell_width == 0 || t.elevation_step == 0 || t.chunk_width == 0 {
        return Err(error(
            origin,
            file,
            "terrain",
            "length scale, cell width, elevation step and chunk width must be positive",
        ));
    }
    // MapSize is legacy f32 metadata; compare its single canonical rounding,
    // while terrain geometry/accounting retain the exact integer dimensions.
    for (actual, count, name) in [
        (map.size.x, t.columns, "size.x"),
        (map.size.y, t.rows, "size.y"),
    ] {
        let width = (f64::from(count) * f64::from(t.cell_width) / f64::from(t.length_scale)) as f32;
        if !actual.is_finite() || actual <= 0.0 || actual.to_bits() != width.to_bits() {
            return Err(error(
                origin,
                file,
                name,
                "must match the terrain's integer grid extent",
            ));
        }
    }
    if t.initial_elevation < t.floor || t.initial_elevation > t.ceiling {
        return Err(error(
            origin,
            file,
            "terrain.initial_elevation",
            "outside floor/ceiling",
        ));
    }
    let count = usize::try_from(cells).map_err(|_| {
        error(
            origin,
            file,
            "terrain",
            "cell count does not fit this target",
        )
    })?;
    if t.initial_heights.len() > count {
        return Err(error(
            origin,
            file,
            "terrain.initial_heights",
            "more patches than cells",
        ));
    }
    let mut heights = Vec::new();
    heights.try_reserve_exact(count).map_err(|_| {
        error(
            origin,
            file,
            "terrain",
            "cannot allocate public initial heights",
        )
    })?;
    heights.resize(count, t.initial_elevation);
    let mut written = Vec::new();
    written
        .try_reserve_exact(count)
        .map_err(|_| error(origin, file, "terrain", "cannot allocate patch validation"))?;
    written.resize(count, false);
    for (i, patch) in t.initial_heights.iter().enumerate() {
        let path = format!("terrain.initial_heights[{i}]");
        rectangle(origin, file, &path, patch.cells, t.columns, t.rows)?;
        if patch.elevation < t.floor || patch.elevation > t.ceiling {
            return Err(error(origin, file, &path, "height outside floor/ceiling"));
        }
        for y in patch.cells.min.y..patch.cells.max.y {
            for x in patch.cells.min.x..patch.cells.max.x {
                let index = y as usize * t.columns as usize + x as usize;
                if written[index] {
                    return Err(error(origin, file, &path, "initial height patches overlap"));
                }
                written[index] = true;
                heights[index] = patch.elevation;
            }
        }
    }
    let maximum = heights.iter().copied().max().unwrap_or(t.floor);
    let TerrainGenerationDef::Version1 { bands } = &t.generation;
    let mut expected = t.floor;
    let mut choices = 0u64;
    for (i, band) in bands.iter().enumerate() {
        let path = format!("terrain.generation.bands[{i}]");
        if band.bottom != expected
            || band.top <= band.bottom
            || band.top > maximum
            || band.choices.is_empty()
        {
            return Err(error(
                origin,
                file,
                &path,
                "bands must cover floor through the maximum initial surface contiguously",
            ));
        }
        let mut total = 0u64;
        let mut seen = BTreeSet::new();
        for choice in &band.choices {
            checked_content_ref(origin, file, &path, &choice.material)?;
            if choice.weight == 0 || !seen.insert(&choice.material) {
                return Err(error(
                    origin,
                    file,
                    &path,
                    "material choices require distinct references and positive weights",
                ));
            }
            total = total
                .checked_add(choice.weight)
                .ok_or_else(|| error(origin, file, &path, "weight total overflow"))?;
        }
        choices = choices
            .checked_add(band.choices.len() as u64)
            .ok_or_else(|| error(origin, file, &path, "choice count overflow"))?;
        if choices > material::MAX_MATERIAL_STEPS {
            return Err(error(
                origin,
                file,
                &path,
                "too many public material choices",
            ));
        }
        expected = band.top;
    }
    if expected != maximum {
        return Err(error(
            origin,
            file,
            "terrain.generation",
            "bands leave a gap below the initial surface",
        ));
    }
    Ok(Some(heights))
}
/// Validate public map geometry even when no final merged catalog is available.
pub fn validate_terrain_definition(
    origin: &PackageId,
    file: &str,
    map: &MapDef,
) -> Result<(), ModError> {
    initial_elevations(origin, file, map).map(|_| ())
}
/// Validate and resolve the opted-in map against the FINAL merged catalog. This
/// returns public generator rules only; the simulation supplies its private seed.
pub fn validate_terrain_map(
    origin: &PackageId,
    file: &str,
    map: &MapDef,
    catalog: &ContentCatalog,
) -> Result<Option<ResolvedTerrainDef>, ModError> {
    if map.terrain.is_some()
        && origin
            .0
            .len()
            .saturating_add(1)
            .saturating_add(map.id.len())
            > MAX_VISIBILITY_MAP_ID_BYTES
    {
        return Err(error(
            origin,
            file,
            "id",
            "qualified terrain map identity exceeds 512 UTF-8 bytes for atomic state framing",
        ));
    }
    let Some(heights) = initial_elevations(origin, file, map)? else {
        return Ok(None);
    };
    validate_terrain_catalog(origin, catalog)?;
    let t = map
        .terrain
        .as_ref()
        .ok_or_else(|| error(origin, file, "terrain", "terrain disappeared"))?;
    let prism = material::Prism::new(
        u64::from(t.length_scale),
        u64::from(t.cell_width),
        u64::from(t.elevation_step),
    )
    .map_err(|e| error(origin, file, "terrain", format!("invalid prism: {e:?}")))?;
    for m in catalog.terrain_materials.values() {
        prism
            .work(pure_ratio(m.work_per_world_cube), u64::MAX)
            .map_err(|e| {
                error(
                    origin,
                    source(catalog, &m.id),
                    "work_per_world_cube",
                    format!("invalid prism work: {e:?}"),
                )
            })?;
        for row in m.natural_outputs.iter().chain(&m.fill_inputs) {
            prism
                .amount(pure_ratio(row.per_world_cube), u64::MAX)
                .map_err(|e| {
                    error(
                        origin,
                        source(catalog, &m.id),
                        "per_world_cube",
                        format!("invalid prism amount: {e:?}"),
                    )
                })?;
        }
    }
    for unit in catalog.units.values() {
        for tool in unit
            .terrain_tools
            .iter()
            .filter_map(|id| catalog.terrain_tools.get(id))
        {
            let materials: Vec<_> = match &tool.operation {
                ResolvedTerrainOperation::Excavate => tool
                    .allowed_materials
                    .iter()
                    .filter_map(|id| catalog.terrain_materials.get(id))
                    .collect(),
                ResolvedTerrainOperation::Fill { material } => catalog
                    .terrain_materials
                    .get(material)
                    .into_iter()
                    .collect(),
            };
            for material in materials {
                let rates = match tool.operation {
                    ResolvedTerrainOperation::Excavate
                        if tool.output_destination == TerrainOutputDestination::Inventory =>
                    {
                        &material.natural_outputs
                    }
                    ResolvedTerrainOperation::Fill { .. } => &material.fill_inputs,
                    _ => continue,
                };
                let mut total = 0u64;
                for row in rates {
                    if catalog
                        .resources
                        .get(&row.resource)
                        .is_some_and(|resource| resource.kind == ResourceKind::Item)
                    {
                        let amount = prism
                            .amount(pure_ratio(row.per_world_cube), u64::MAX)
                            .map_err(|e| {
                                error(
                                    origin,
                                    source(catalog, &unit.id),
                                    "terrain_tools",
                                    format!("invalid prism resource: {e:?}"),
                                )
                            })?;
                        total = total.checked_add(amount).ok_or_else(|| {
                            error(
                                origin,
                                source(catalog, &unit.id),
                                "inventory.capacity",
                                "one prism's Item total overflows",
                            )
                        })?;
                    }
                }
                if total > 0
                    && !unit
                        .inventory
                        .as_ref()
                        .is_some_and(|inventory| total <= inventory.capacity)
                {
                    return Err(error(
                        origin,
                        source(catalog, &unit.id),
                        "inventory.capacity",
                        "one complete prism cannot fit in the actor inventory",
                    ));
                }
            }
        }
    }
    let TerrainGenerationDef::Version1 { bands } = &t.generation;
    let generation = bands
        .iter()
        .enumerate()
        .map(|(i, band)| {
            let choices = band
                .choices
                .iter()
                .map(|choice| {
                    let id = checked_content_ref(
                        origin,
                        file,
                        &format!("terrain.generation.bands[{i}]"),
                        &choice.material,
                    )?;
                    require(
                        origin,
                        file,
                        "terrain.generation",
                        &id,
                        &catalog.terrain_materials,
                        None,
                    )?;
                    Ok(ResolvedTerrainWeightedMaterial {
                        material: id,
                        weight: choice.weight,
                    })
                })
                .collect::<Result<Vec<_>, ModError>>()?;
            Ok(ResolvedTerrainDatumBand {
                bottom: band.bottom,
                top: band.top,
                choices,
            })
        })
        .collect::<Result<Vec<_>, ModError>>()?;
    let grid = geometry::Grid::checked(
        t.columns,
        t.rows,
        t.length_scale,
        t.cell_width,
        t.elevation_step,
        t.floor,
        t.ceiling,
    )
    .map_err(|e| error(origin, file, "terrain", format!("invalid grid: {e:?}")))?;
    let surface = |cell: geometry::Cell| {
        heights
            .get(cell.y as usize * t.columns as usize + cell.x as usize)
            .copied()
    };
    for (i, placement) in map.placements.iter().enumerate() {
        let path = format!("placements[{i}]");
        let id = checked_content_ref(origin, file, &format!("{path}.unit"), &placement.unit)?;
        let unit = catalog
            .units
            .get(&id)
            .ok_or_else(|| error(origin, file, &path, "unknown placed unit"))?;
        let max_step = unit
            .terrain_class
            .as_ref()
            .and_then(|id| catalog.terrain_movement_classes.get(id))
            .map_or(0, |class| class.max_step);
        if unit.movement.is_none() && unit.terrain_foundation != Some(TerrainFoundation::ForbidEdit)
        {
            return Err(error(
                origin,
                file,
                &path,
                "static terrain placement requires explicit ForbidEdit foundation",
            ));
        }
        let Footprint::Circle { radius } = unit.footprint;
        grid.disk_support(
            &surface,
            geometry::Point {
                x: f64::from(placement.pos.x),
                y: f64::from(placement.pos.y),
            },
            f64::from(radius),
            if unit.movement.is_none() { 0 } else { max_step },
            &mut geometry::Budget::new(2_097_152),
        )
        .map_err(|e| {
            error(
                origin,
                file,
                &path,
                format!("unsupported initial footprint: {e:?}"),
            )
        })?;
    }
    Ok(Some(ResolvedTerrainDef {
        length_scale: t.length_scale,
        cell_width: t.cell_width,
        elevation_step: t.elevation_step,
        columns: t.columns,
        rows: t.rows,
        chunk_width: t.chunk_width,
        initial_elevation: t.initial_elevation,
        initial_heights: t.initial_heights.clone(),
        floor: t.floor,
        ceiling: t.ceiling,
        generation,
    }))
}
/// Work target admission bounds are independent of hidden material and occupancy.
pub fn validate_designation(
    origin: &PackageId,
    file: &str,
    terrain: &ResolvedTerrainDef,
    designation: &TerrainDesignation,
    catalog: &ContentCatalog,
) -> Result<(), ModError> {
    if rectangle(
        origin,
        file,
        "cells",
        designation.cells,
        terrain.columns,
        terrain.rows,
    )? > 256
    {
        return Err(error(
            origin,
            file,
            "cells",
            "a designation may contain at most 256 cells",
        ));
    }
    if designation.target_elevation < terrain.floor
        || designation.target_elevation > terrain.ceiling
    {
        return Err(error(
            origin,
            file,
            "target_elevation",
            "outside terrain bounds",
        ));
    }
    let tool = checked_content_ref(origin, file, "tool", &designation.tool)?;
    require(origin, file, "tool", &tool, &catalog.terrain_tools, None)
}

/// A terrain tool list binds an exact declared order; it never selects the first
/// matching order or invents an identifier from the tool's name.
pub(crate) fn validate_tool_source_order(
    origin: &PackageId,
    file: &str,
    field: &str,
    order: &OrderDef,
) -> Result<(), ModError> {
    if order.behaviour != Behaviour::TerrainWork || order.target != TargetKind::TerrainDesignation {
        return Err(error(
            origin,
            file,
            field,
            "TerrainTools requires a TerrainWork order with TerrainDesignation target",
        ));
    }
    Ok(())
}
/// Complete-catalog validation of tool-list bindings, including declared library
/// references that package-local interface validation necessarily defers.
pub fn validate_terrain_tool_sources(
    origin: &PackageId,
    file: &str,
    interface: &InterfaceDef,
    catalog: &ContentCatalog,
) -> Result<(), ModError> {
    fn widget(
        origin: &PackageId,
        file: &str,
        path: &str,
        row: &Widget,
        catalog: &ContentCatalog,
    ) -> Result<(), ModError> {
        match &row.widget {
            Primitive::List { source, item, .. } | Primitive::Grid { source, item, .. } => {
                if let Source::TerrainTools { order } = source {
                    let field = format!("{path}.widget.source.order");
                    let id = checked_content_ref(origin, file, &field, order)?;
                    let definition = catalog.orders.get(&id).ok_or_else(|| {
                        error(
                            origin,
                            file,
                            &field,
                            format!("unknown terrain work order `{id}`"),
                        )
                    })?;
                    validate_tool_source_order(origin, file, &field, definition)?;
                }
                widget(origin, file, &format!("{path}.item"), item, catalog)?;
            }
            Primitive::Box { children, .. } => {
                for (index, row) in children.iter().enumerate() {
                    widget(
                        origin,
                        file,
                        &format!("{path}.children[{index}]"),
                        row,
                        catalog,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    for (index, row) in interface.root.iter().enumerate() {
        widget(origin, file, &format!("root[{index}]"), row, catalog)?;
    }
    Ok(())
}
