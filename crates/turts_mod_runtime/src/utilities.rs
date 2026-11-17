//! API 0.2.18 (M6) public utility authoring validation. A utility type is content: it names the
//! `Supply` resource of scope `Network` that it carries, the surface ports of unit types, the
//! burial depth, the reach, the segment limits, the cost, the exposure policy and the overlay look.
//! Topology is server state; nothing here reads or writes it.
use crate::content::{checked_content_ref, error};
use crate::terrain::insert_source;
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::*;

/// A port offset may lie this far outside the footprint circle, in world units (spec M6,
/// "Public authoring").
const PORT_MARGIN: f32 = 2.0;

/// The engine colour syntax of authored colours: `#RRGGBB`. It is the check that
/// `terrain_materials[].surface_color` uses.
fn colour(origin: &PackageId, file: &str, field: &str, value: &str) -> Result<(), ModError> {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err(error(origin, file, field, "must be #RRGGBB"));
    }
    Ok(())
}

/// A reference to content of this package. A utility type never names foreign content.
fn local_ref(
    origin: &PackageId,
    file: &str,
    field: &str,
    text: &str,
) -> Result<ContentId, ModError> {
    let id = checked_content_ref(origin, file, field, text)?;
    if id.package != *origin {
        return Err(error(
            origin,
            file,
            field,
            "must name content of this package",
        ));
    }
    Ok(id)
}

fn source(catalog: &ContentCatalog, id: &ContentId) -> String {
    catalog
        .source_files
        .get(id)
        .map_or_else(|| "content".to_string(), Clone::clone)
}

fn network_resource(catalog: &ContentCatalog, id: &str) -> Option<ContentId> {
    let id = ContentId::parse(id).ok()?;
    match catalog.resources.get(&id)?.kind {
        ResourceKind::Supply {
            scope: SupplyScope::Network,
            ..
        } => Some(id),
        _ => None,
    }
}

/// Resolve and check the utility types of one package, then check every use of a Network
/// resource in the units and orders of the package. The first problem is the error.
pub fn install_utilities(
    manifest: &Manifest,
    catalog: &mut ContentCatalog,
    utilities: Vec<(String, UtilityDef)>,
) -> Result<(), ModError> {
    let origin = PackageId::parse(&manifest.id).map_err(|message| {
        error(
            &PackageId(manifest.id.clone()),
            "manifest.ron",
            "id",
            message,
        )
    })?;
    let mut carried_by: BTreeMap<ContentId, ContentId> = BTreeMap::new();
    for (file, value) in utilities {
        let resolved = resolve(&origin, &file, value, catalog, &mut carried_by)?;
        catalog.utilities.insert(resolved.id.clone(), resolved);
    }
    // Every Network resource of the package has exactly one utility type.
    for (id, resource) in &catalog.resources {
        if id.package == origin
            && matches!(
                resource.kind,
                ResourceKind::Supply {
                    scope: SupplyScope::Network,
                    ..
                }
            )
            && !carried_by.contains_key(id)
        {
            return Err(error(
                &origin,
                &source(catalog, id),
                "kind.scope",
                "a Network resource needs one utility type that carries it",
            ));
        }
    }
    validate_unit_uses(&origin, catalog)
}

fn resolve(
    origin: &PackageId,
    file: &str,
    value: UtilityDef,
    catalog: &mut ContentCatalog,
    carried_by: &mut BTreeMap<ContentId, ContentId>,
) -> Result<ResolvedUtility, ModError> {
    let id = insert_source(origin, file, &value.id, catalog)?;
    let depth = value.depth;
    if depth.min < 1 {
        return Err(error(origin, file, "depth.min", "must be at least 1"));
    }
    if depth.default < depth.min {
        return Err(error(
            origin,
            file,
            "depth.default",
            format!("must be at least depth.min ({})", depth.min),
        ));
    }
    if depth.max < depth.default {
        return Err(error(
            origin,
            file,
            "depth.max",
            format!("must be at least depth.default ({})", depth.default),
        ));
    }
    if depth.max > MAX_UTILITY_DEPTH {
        return Err(error(
            origin,
            file,
            "depth.max",
            format!("must be at most {MAX_UTILITY_DEPTH}"),
        ));
    }
    if !value.max_segment_length.is_finite()
        || value.max_segment_length <= 0.0
        || value.max_segment_length > MAX_UTILITY_SEGMENT_LENGTH
    {
        return Err(error(
            origin,
            file,
            "max_segment_length",
            format!("must be finite, > 0 and <= {MAX_UTILITY_SEGMENT_LENGTH}"),
        ));
    }
    if !value.reach.is_finite() || value.reach <= 0.0 {
        return Err(error(origin, file, "reach", "must be finite and > 0"));
    }
    let carries = local_ref(origin, file, "carries", &value.carries)?;
    let Some(resource) = catalog.resources.get(&carries) else {
        return Err(error(
            origin,
            file,
            "carries",
            format!("unknown resource `{carries}`"),
        ));
    };
    if !matches!(
        resource.kind,
        ResourceKind::Supply {
            scope: SupplyScope::Network,
            ..
        }
    ) {
        return Err(error(
            origin,
            file,
            "carries",
            format!("`{carries}` must be a Supply resource with scope Network"),
        ));
    }
    if let Some(first) = carried_by.insert(carries.clone(), id.clone()) {
        return Err(error(
            origin,
            file,
            "carries",
            format!("`{carries}` is already carried by utility type `{first}`"),
        ));
    }
    let mut ports = BTreeMap::new();
    for (index, port) in value.ports.iter().enumerate() {
        let path = format!("ports[{index}]");
        let unit_id = local_ref(origin, file, &format!("{path}.unit"), &port.unit)?;
        let Some(unit) = catalog.units.get(&unit_id) else {
            return Err(error(
                origin,
                file,
                &format!("{path}.unit"),
                format!("unknown unit type `{unit_id}`"),
            ));
        };
        if ports.contains_key(&unit_id) {
            return Err(error(
                origin,
                file,
                &format!("{path}.unit"),
                format!("second port for unit type `{unit_id}`"),
            ));
        }
        if unit.movement.is_some() {
            return Err(error(
                origin,
                file,
                &format!("{path}.unit"),
                format!("`{unit_id}` has `movement`; a port needs a unit type that cannot move"),
            ));
        }
        let Footprint::Circle { radius } = unit.footprint;
        let reach = radius + PORT_MARGIN;
        let (x, y) = (port.offset.x, port.offset.y);
        if !x.is_finite() || !y.is_finite() || x.hypot(y) > reach {
            return Err(error(
                origin,
                file,
                &format!("{path}.offset"),
                format!(
                    "must be finite and within {reach} of the unit centre (footprint radius {radius} + {PORT_MARGIN})"
                ),
            ));
        }
        ports.insert(unit_id, port.offset);
    }
    let mut segment_cost = Vec::new();
    let mut paid = BTreeSet::new();
    for (index, cost) in value.segment_cost.iter().enumerate() {
        let path = format!("segment_cost[{index}]");
        let resource = local_ref(origin, file, &format!("{path}.resource"), &cost.resource)?;
        match catalog.resources.get(&resource).map(|row| row.kind) {
            None => {
                return Err(error(
                    origin,
                    file,
                    &format!("{path}.resource"),
                    format!("unknown resource `{resource}`"),
                ));
            }
            Some(ResourceKind::Balance) => {}
            Some(_) => {
                return Err(error(
                    origin,
                    file,
                    &format!("{path}.resource"),
                    format!("`{resource}` must be a Balance resource"),
                ));
            }
        }
        if cost.amount == 0 {
            return Err(error(
                origin,
                file,
                &format!("{path}.amount"),
                "must be positive; omit a zero row",
            ));
        }
        if !paid.insert(resource.clone()) {
            return Err(error(
                origin,
                file,
                &format!("{path}.resource"),
                format!("second entry for `{resource}`"),
            ));
        }
        segment_cost.push((resource, cost.amount));
    }
    colour(origin, file, "appearance.colour", &value.appearance.colour)?;
    colour(
        origin,
        file,
        "appearance.exposed_colour",
        &value.appearance.exposed_colour,
    )?;
    Ok(ResolvedUtility {
        id,
        name: value.name,
        carries,
        ports,
        depth,
        reach: value.reach,
        max_segment_length: value.max_segment_length,
        segment_cost,
        exposure: value.exposure,
        appearance: value.appearance,
    })
}

/// The ports of the unit type of each utility, for the checks of the units.
fn has_port(catalog: &ContentCatalog, unit: &ContentId, resource: &ContentId) -> bool {
    catalog
        .utilities
        .values()
        .any(|utility| &utility.carries == resource && utility.ports.contains_key(unit))
}

/// A unit that provides or consumes a Network resource needs a port of the utility that carries
/// it. A producer or weapon `when_short` on a Network resource needs the same port. A constructor
/// has no port (a site is not a unit), so its `when_short` cannot name a Network resource.
fn validate_unit_uses(origin: &PackageId, catalog: &ContentCatalog) -> Result<(), ModError> {
    for unit in catalog.units.values() {
        let file = source(catalog, &unit.id);
        for (field, rows) in [("provides", &unit.provides), ("consumes", &unit.consumes)] {
            for (index, row) in rows.iter().enumerate() {
                let Some(resource) = network_resource(catalog, &row.resource) else {
                    continue;
                };
                if !has_port(catalog, &unit.id, &resource) {
                    return Err(error(
                        origin,
                        &file,
                        &format!("{field}[{index}].resource"),
                        format!(
                            "`{}` has no port of the utility type that carries `{resource}`",
                            unit.id
                        ),
                    ));
                }
            }
        }
        let lists = [
            unit.producer
                .as_ref()
                .map(|row| ("producer", row.when_short.as_slice())),
            unit.weapon
                .as_ref()
                .map(|row| ("weapon", row.when_short.as_slice())),
            unit.constructor
                .as_ref()
                .map(|row| ("constructor", row.when_short.as_slice())),
        ];
        for (owner, rows) in lists.into_iter().flatten() {
            for (index, row) in rows.iter().enumerate() {
                let Some(resource) = network_resource(catalog, &row.resource) else {
                    continue;
                };
                let path = format!("{owner}.when_short[{index}].resource");
                if owner == "constructor" {
                    return Err(error(
                        origin,
                        &file,
                        &path,
                        format!("a constructor has no port; `{resource}` is a Network resource"),
                    ));
                }
                if !has_port(catalog, &unit.id, &resource) {
                    return Err(error(
                        origin,
                        &file,
                        &path,
                        format!(
                            "`{}` has no port of the utility type that carries `{resource}`",
                            unit.id
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// A `UtilityRoutes` list binds one exact declared order. It never selects the first matching
/// order. Complete-catalog check, like `validate_terrain_tool_sources`.
pub fn validate_utility_sources(
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
                if let Source::UtilityRoutes { order } = source {
                    let field = format!("{path}.widget.source.order");
                    let id = checked_content_ref(origin, file, &field, order)?;
                    let definition = catalog.orders.get(&id).ok_or_else(|| {
                        error(
                            origin,
                            file,
                            &field,
                            format!("unknown utility route order `{id}`"),
                        )
                    })?;
                    if definition.behaviour != Behaviour::UtilityRoute {
                        return Err(error(
                            origin,
                            file,
                            &field,
                            "UtilityRoutes requires an order with behaviour UtilityRoute",
                        ));
                    }
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
