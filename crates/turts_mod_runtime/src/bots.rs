//! Portable bot validation. A client need not link a profile's host controller factory.
use crate::content::{error, local_id};
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::{
    BotDef, BotGoalAction, BotRecipeKind, BotValue, Capability, ContentCatalog, ContentId,
    ModError, PackageId,
};

const MAX_CONFIG_DEPTH: usize = 16;
const MAX_CONFIG_ITEMS: usize = 4096;
const MAX_CONFIG_BYTES: usize = 65536;

/// Validate profiles against their complete available catalog. References remain portable.
/// Factory-specific configuration is validated later by the host registry.
pub fn validate_bots(
    origin: &PackageId,
    sources: Vec<(String, BotDef)>,
    catalog: &ContentCatalog,
) -> Result<BTreeMap<ContentId, BotDef>, ModError> {
    let mut result = BTreeMap::new();
    for (file, bot) in sources {
        let id = local_id(origin, &file, &bot.id)?;
        if result.contains_key(&id)
            || catalog.units.contains_key(&id)
            || catalog.production.contains_key(&id)
            || catalog.resources.contains_key(&id)
            || catalog.orders.contains_key(&id)
        {
            return Err(error(origin, &file, "id", "duplicate content id"));
        }
        validate_bot(origin, &file, &bot, catalog)?;
        result.insert(id, bot);
    }
    Ok(result)
}

fn validate_bot(
    origin: &PackageId,
    file: &str,
    bot: &BotDef,
    catalog: &ContentCatalog,
) -> Result<(), ModError> {
    let fail = |path: &str, message: &str| error(origin, file, path, message);
    if bot.name.trim().is_empty() {
        return Err(fail("name", "must not be empty"));
    }
    if bot.decision_ms == 0 {
        return Err(fail("decision_ms", "must be positive"));
    }
    if bot.controllers.is_empty() || bot.controllers.len() > 32 {
        return Err(fail("controllers", "must contain 1 to 32 instances"));
    }
    let mut keys = BTreeSet::new();
    for (i, c) in bot.controllers.iter().enumerate() {
        let path = format!("controllers[{i}]");
        key(origin, file, &format!("{path}.key"), &c.key, &mut keys)?;
        reference(origin, file, &format!("{path}.factory"), &c.factory)?;
        let mut count = 0;
        let mut bytes = 0;
        validate_value(
            origin,
            file,
            &format!("{path}.config"),
            &c.config,
            0,
            &mut count,
            &mut bytes,
        )?;
        let encoded_bytes = serde_json::to_vec(&c.config)
            .map_err(|_| fail(&format!("{path}.config"), "configuration cannot be encoded"))?
            .len();
        if encoded_bytes > MAX_CONFIG_BYTES {
            return Err(fail(
                &format!("{path}.config"),
                "encoded configuration exceeds 65536 bytes",
            ));
        }
        if c.quota.queries == 0
            || c.quota.candidates == 0
            || c.quota.rows == 0
            || c.quota.proposals == 0
        {
            return Err(fail(
                &format!("{path}.quota"),
                "all quota dimensions must be positive",
            ));
        }
    }
    let k = &bot.knowledge;
    let mut roles = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (i, role) in k.roles.iter().enumerate() {
        let path = format!("knowledge.roles[{i}]");
        key(origin, file, &format!("{path}.key"), &role.key, &mut seen)?;
        if role.units.is_empty() {
            return Err(fail(&format!("{path}.units"), "must not be empty"));
        }
        let mut types = BTreeSet::new();
        for (j, unit) in role.units.iter().enumerate() {
            let field = format!("{path}.units[{j}]");
            let id = reference(origin, file, &field, unit)?;
            if !types.insert(id.clone()) {
                return Err(fail(&field, "duplicate unit reference"));
            }
            let definition = catalog
                .units
                .get(&id)
                .ok_or_else(|| fail(&field, "unknown unit"))?;
            for cap in &role.requires {
                if !has_capability(definition, *cap) {
                    return Err(fail(
                        &field,
                        "unit does not satisfy the required capability",
                    ));
                }
            }
        }
        roles.insert(role.key.as_str(), role);
    }
    let mut orders = BTreeMap::new();
    seen.clear();
    for (i, binding) in k.orders.iter().enumerate() {
        let path = format!("knowledge.orders[{i}]");
        key(
            origin,
            file,
            &format!("{path}.key"),
            &binding.key,
            &mut seen,
        )?;
        let role = roles
            .get(binding.actor_role.as_str())
            .ok_or_else(|| fail(&format!("{path}.actor_role"), "unknown role"))?;
        let id = reference(origin, file, &format!("{path}.order"), &binding.order)?;
        let order = catalog
            .orders
            .get(&id)
            .ok_or_else(|| fail(&format!("{path}.order"), "unknown order"))?;
        for unit in &role.units {
            let def = &catalog.units[&ContentId::parse(unit).expect("validated reference")];
            let required = match order.behaviour {
                turts_mod_api::Behaviour::Move => Some(Capability::Movement),
                turts_mod_api::Behaviour::Attack => Some(Capability::Weapon),
                turts_mod_api::Behaviour::Gather => Some(Capability::Gatherer),
                turts_mod_api::Behaviour::Produce => Some(Capability::Producer),
                turts_mod_api::Behaviour::Construct => Some(Capability::Constructor),
                turts_mod_api::Behaviour::Repair => Some(Capability::Repairer),
                _ => None,
            };
            if (!order.units.is_empty() && !order.units.contains(unit))
                || required.is_some_and(|cap| !has_capability(def, cap))
            {
                return Err(fail(
                    &format!("{path}.actor_role"),
                    "role cannot issue this order",
                ));
            }
        }
        orders.insert(binding.key.as_str(), binding);
    }
    let mut recipes = BTreeMap::new();
    seen.clear();
    for (i, binding) in k.recipes.iter().enumerate() {
        let path = format!("knowledge.recipes[{i}]");
        key(
            origin,
            file,
            &format!("{path}.key"),
            &binding.key,
            &mut seen,
        )?;
        let role = roles
            .get(binding.actor_role.as_str())
            .ok_or_else(|| fail(&format!("{path}.actor_role"), "unknown role"))?;
        let order = orders
            .get(binding.order.as_str())
            .ok_or_else(|| fail(&format!("{path}.order"), "unknown order binding"))?;
        if order.actor_role != binding.actor_role {
            return Err(fail(
                &format!("{path}.order"),
                "order and recipe actor roles differ",
            ));
        }
        let id = reference(origin, file, &format!("{path}.recipe"), &binding.recipe)?;
        if !catalog.production.contains_key(&id) {
            return Err(fail(&format!("{path}.recipe"), "unknown production recipe"));
        }
        for unit in &role.units {
            let def = &catalog.units[&ContentId::parse(unit).expect("validated reference")];
            let eligible = match binding.kind {
                BotRecipeKind::Produce => def
                    .producer
                    .as_ref()
                    .is_some_and(|p| p.recipes.contains(&id)),
                BotRecipeKind::Construct => def
                    .constructor
                    .as_ref()
                    .is_some_and(|c| c.buildings.contains(&id)),
            };
            if !eligible {
                return Err(fail(
                    &format!("{path}.actor_role"),
                    "actor cannot use this recipe",
                ));
            }
        }
        let expected = match binding.kind {
            BotRecipeKind::Produce => turts_mod_api::Behaviour::Produce,
            BotRecipeKind::Construct => turts_mod_api::Behaviour::Construct,
        };
        if catalog.orders[&ContentId::parse(&order.order).expect("validated order")].behaviour
            != expected
        {
            return Err(fail(
                &format!("{path}.order"),
                "order behaviour does not match recipe kind",
            ));
        }
        recipes.insert(binding.key.as_str(), binding);
    }
    let check_output = |path: &str, role: &str, recipe: &str| -> Result<(), ModError> {
        let role = roles
            .get(role)
            .ok_or_else(|| fail(&format!("{path}.role"), "unknown role"))?;
        let recipe = recipes
            .get(recipe)
            .ok_or_else(|| fail(&format!("{path}.recipe"), "unknown recipe binding"))?;
        let output = &catalog.production
            [&ContentId::parse(&recipe.recipe).expect("validated recipe")]
            .output_unit;
        if !role.units.iter().any(|u| u == &output.to_string()) {
            return Err(fail(
                &format!("{path}.recipe"),
                "recipe output does not satisfy target role",
            ));
        }
        Ok(())
    };
    seen.clear();
    for (group, goals) in [("opening", &k.opening), ("recovery", &k.recovery)] {
        for (i, goal) in goals.iter().enumerate() {
            let path = format!("knowledge.{group}[{i}]");
            key(origin, file, &format!("{path}.key"), &goal.key, &mut seen)?;
            match &goal.action {
                BotGoalAction::EnsureUnits {
                    role,
                    count,
                    recipe,
                } => {
                    if *count == 0 {
                        return Err(fail(&format!("{path}.count"), "must be positive"));
                    }
                    check_output(&path, role, recipe)?;
                }
                BotGoalAction::Gather {
                    role,
                    resource,
                    order,
                } => {
                    let r = roles
                        .get(role.as_str())
                        .ok_or_else(|| fail(&format!("{path}.role"), "unknown role"))?;
                    let resource = reference(origin, file, &format!("{path}.resource"), resource)?;
                    if !catalog.resources.contains_key(&resource) {
                        return Err(fail(&format!("{path}.resource"), "unknown resource"));
                    }
                    let binding = orders
                        .get(order.as_str())
                        .ok_or_else(|| fail(&format!("{path}.order"), "unknown order binding"))?;
                    if binding.actor_role != *role
                        || catalog.orders
                            [&ContentId::parse(&binding.order).expect("validated order")]
                            .behaviour
                            != turts_mod_api::Behaviour::Gather
                    {
                        return Err(fail(
                            &format!("{path}.order"),
                            "requires a Gather order for this role",
                        ));
                    }
                    for u in &r.units {
                        if !catalog.units[&ContentId::parse(u).expect("validated")]
                            .gatherer
                            .as_ref()
                            .is_some_and(|g| g.items.contains(&resource))
                        {
                            return Err(fail(
                                &format!("{path}.resource"),
                                "gatherer cannot collect this resource",
                            ));
                        }
                    }
                }
                BotGoalAction::EnsureSupply {
                    resource,
                    amount,
                    provider_role,
                    recipe,
                } => {
                    if *amount == 0 {
                        return Err(fail(&format!("{path}.amount"), "must be positive"));
                    }
                    let resource = reference(origin, file, &format!("{path}.resource"), resource)?;
                    if !catalog.resources.contains_key(&resource) {
                        return Err(fail(&format!("{path}.resource"), "unknown resource"));
                    }
                    check_output(&path, provider_role, recipe)?;
                    for u in &roles[provider_role.as_str()].units {
                        if !catalog.units[&ContentId::parse(u).expect("validated")]
                            .provides
                            .iter()
                            .any(|p| p.resource == resource.to_string() && p.amount > 0)
                        {
                            return Err(fail(
                                &format!("{path}.provider_role"),
                                "role does not provide this supply",
                            ));
                        }
                    }
                }
            }
        }
    }
    for (i, target) in k.production.iter().enumerate() {
        let path = format!("knowledge.production[{i}]");
        key(origin, file, &format!("{path}.key"), &target.key, &mut seen)?;
        if target.count == 0 {
            return Err(fail(&format!("{path}.count"), "must be positive"));
        }
        check_output(&path, &target.role, &target.recipe)?;
    }
    for (i, squad) in k.squads.iter().enumerate() {
        let path = format!("knowledge.squads[{i}]");
        key(origin, file, &format!("{path}.key"), &squad.key, &mut seen)?;
        for role in std::iter::once(&squad.role)
            .chain(std::iter::once(&squad.anchor_role))
            .chain(&squad.target_roles)
        {
            if !roles.contains_key(role.as_str()) {
                return Err(fail(&format!("{path}.role"), "unknown role"));
            }
        }
        if squad.min_units == 0
            || !squad.defend_radius.is_finite()
            || squad.defend_radius < 0.0
            || !squad.assemble_radius.is_finite()
            || squad.assemble_radius < 0.0
        {
            return Err(fail(
                &path,
                "requires positive units and finite nonnegative radii",
            ));
        }
        for (key, behavior) in [
            (&squad.attack_order, turts_mod_api::Behaviour::Attack),
            (&squad.move_order, turts_mod_api::Behaviour::Move),
        ] {
            let binding = orders
                .get(key.as_str())
                .ok_or_else(|| fail(&path, "unknown squad order binding"))?;
            if binding.actor_role != squad.role
                || catalog.orders[&ContentId::parse(&binding.order).expect("validated")].behaviour
                    != behavior
            {
                return Err(fail(
                    &path,
                    "squad order has incompatible role or behaviour",
                ));
            }
        }
    }
    Ok(())
}

fn key(
    origin: &PackageId,
    file: &str,
    path: &str,
    value: &str,
    seen: &mut BTreeSet<String>,
) -> Result<(), ModError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-./".contains(&b))
    {
        return Err(error(
            origin,
            file,
            path,
            "requires a stable key of 1 to 128 ASCII letters, digits, _, -, . or /",
        ));
    }
    if !seen.insert(value.to_owned()) {
        return Err(error(origin, file, path, "duplicate authored key"));
    }
    Ok(())
}
fn reference(
    origin: &PackageId,
    file: &str,
    path: &str,
    value: &str,
) -> Result<ContentId, ModError> {
    crate::content::checked_content_ref(origin, file, path, value)
}
fn has_capability(unit: &turts_mod_api::content::ResolvedUnit, cap: Capability) -> bool {
    match cap {
        Capability::Movement => unit.movement.is_some(),
        Capability::Weapon => unit.weapon.is_some(),
        Capability::Gatherer => unit.gatherer.is_some(),
        Capability::Producer => unit.producer.is_some(),
        Capability::Research => unit.research.is_some(),
        Capability::Constructor => unit.constructor.is_some(),
        Capability::Repairer => unit.repairer.is_some(),
    }
}
fn validate_value(
    origin: &PackageId,
    file: &str,
    path: &str,
    value: &BotValue,
    depth: usize,
    count: &mut usize,
    bytes: &mut usize,
) -> Result<(), ModError> {
    *count += 1;
    if depth > MAX_CONFIG_DEPTH || *count > MAX_CONFIG_ITEMS {
        return Err(error(
            origin,
            file,
            path,
            "configuration exceeds depth or item limit",
        ));
    }
    match value {
        BotValue::String(s) => *bytes += s.len(),
        BotValue::List(values) => {
            for (i, v) in values.iter().enumerate() {
                validate_value(
                    origin,
                    file,
                    &format!("{path}[{i}]"),
                    v,
                    depth + 1,
                    count,
                    bytes,
                )?;
            }
        }
        BotValue::Map(values) => {
            for (k, v) in &values.0 {
                *bytes += k.len();
                validate_value(
                    origin,
                    file,
                    &format!("{path}.{k}"),
                    v,
                    depth + 1,
                    count,
                    bytes,
                )?;
            }
        }
        _ => *bytes += 8,
    }
    if *bytes > MAX_CONFIG_BYTES {
        return Err(error(
            origin,
            file,
            path,
            "configuration exceeds 65536 bytes",
        ));
    }
    Ok(())
}
