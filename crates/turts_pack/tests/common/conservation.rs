use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use turts_mod_api::{
    CancelRefunds, ContentId, CostSource, Manifest, NodeDef, RecipeDef, ResourceDef, ResourceKind,
    TechnologyDef, UnitDef,
};
use turts_sim::{
    AuditState, EconomyEventKind, JobStage, OperationAudit, ReservationClaimView,
    ReservationEndpointView,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFailure {
    pub invariant: &'static str,
    pub detail: String,
}
impl CheckFailure {
    fn new(invariant: &'static str, detail: impl Into<String>) -> Self {
        Self {
            invariant,
            detail: detail.into(),
        }
    }
    pub fn named(invariant: &'static str) -> Self {
        Self::new(invariant, "instrument self-test failure")
    }
}

#[derive(Debug, Clone)]
pub struct OracleCatalog {
    pub package: String,
    pub items: BTreeSet<ContentId>,
    pub nodes: BTreeMap<ContentId, NodeDef>,
    pub units: BTreeMap<ContentId, UnitDef>,
    pub recipes: BTreeMap<ContentId, RecipeDef>,
    pub technologies: BTreeMap<ContentId, TechnologyDef>,
}
impl OracleCatalog {
    pub fn parse(dir: &Path) -> Result<Self, String> {
        let manifest: Manifest = parse_file(&dir.join("manifest.ron"))?;
        let mut result = Self {
            package: manifest.id.clone(),
            items: BTreeSet::new(),
            nodes: BTreeMap::new(),
            units: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
        };
        for path in &manifest.exports.resources {
            let def: ResourceDef = parse_file(&dir.join(path))?;
            if def.kind == ResourceKind::Item {
                result.items.insert(content_id(&manifest.id, &def.id)?);
            }
        }
        for path in &manifest.exports.nodes {
            let def: NodeDef = parse_file(&dir.join(path))?;
            result.nodes.insert(content_id(&manifest.id, &def.id)?, def);
        }
        for path in &manifest.exports.units {
            let def: UnitDef = parse_file(&dir.join(path))?;
            result.units.insert(content_id(&manifest.id, &def.id)?, def);
        }
        for path in &manifest.exports.recipes {
            let def: RecipeDef = parse_file(&dir.join(path))?;
            result
                .recipes
                .insert(content_id(&manifest.id, &def.id)?, def);
        }
        for path in &manifest.exports.technologies {
            let def: TechnologyDef = parse_file(&dir.join(path))?;
            result
                .technologies
                .insert(content_id(&manifest.id, &def.id)?, def);
        }
        Ok(result)
    }
    fn node_item(&self, node: &ContentId) -> Result<ContentId, CheckFailure> {
        let def = self
            .nodes
            .get(node)
            .ok_or_else(|| CheckFailure::new("BAL-ore", format!("unparsed node {node}")))?;
        ContentId::parse(&def.item).map_err(|e| CheckFailure::new("BAL-ore", e))
    }
    pub fn item(&self, item: &str) -> Result<ContentId, CheckFailure> {
        ContentId::parse(item).map_err(|e| CheckFailure::new("SHOT", e))
    }
}
fn parse_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str(&source).map_err(|e| format!("{}: {e}", path.display()))
}
fn content_id(package: &str, id: &str) -> Result<ContentId, String> {
    let qualified = if id.contains(':') {
        id.to_owned()
    } else {
        format!("{package}:{id}")
    };
    ContentId::parse(&qualified).map_err(|e| e.to_string())
}

#[derive(Debug, Default, Clone)]
pub struct OracleHistory {
    pub completed_by_slot: BTreeMap<u8, BTreeSet<ContentId>>,
    pub destroyed: BTreeSet<u32>,
    pub health: BTreeMap<u32, u64>,
    started_jobs: BTreeSet<u64>,
    destroyed_events: BTreeSet<u32>,
}
impl OracleHistory {
    pub fn seed(state: &AuditState) -> Self {
        Self {
            health: state
                .raw_healths
                .iter()
                .map(|r| (r.unit, r.current))
                .collect(),
            ..Self::default()
        }
    }
}

fn named<T>(name: &'static str, result: Result<T, String>) -> Result<T, CheckFailure> {
    result.map_err(|e| CheckFailure::new(name, e))
}
fn add(total: &mut u128, amount: u64, check: &'static str) -> Result<(), CheckFailure> {
    *total = total
        .checked_add(u128::from(amount))
        .ok_or_else(|| CheckFailure::new(check, "stock sum overflow"))?;
    Ok(())
}
pub fn stock(
    state: &AuditState,
    catalog: &OracleCatalog,
    item: &ContentId,
) -> Result<u128, CheckFailure> {
    let mut total = 0_u128;
    for queue in &state.raw_queues {
        for job in &queue.jobs {
            let reserved_technology = catalog.technologies.get(&job.recipe).is_some_and(|tech| {
                tech.payment == turts_mod_api::PaymentPolicy::ReserveThenCharge
            });
            let active = state.ledger_claims.iter().filter(|claim| matches!(claim, ReservationClaimView::Active { map_job_id, .. } if *map_job_id == job.id)).count();
            if reserved_technology && job.stage == JobStage::Queued && active != 1 {
                return Err(CheckFailure::new(
                    "RES",
                    format!(
                        "queued reserve-then-charge job {} has {active} active claims",
                        job.id
                    ),
                ));
            }
            if job.stage != JobStage::Queued && active != 0 {
                return Err(CheckFailure::new(
                    "RES",
                    format!("started job {} still has an active claim", job.id),
                ));
            }
        }
    }
    for balance in &state.economy.balances {
        if &balance.resource == item {
            add(&mut total, balance.on_hand, "BAL-ore")?;
        }
    }
    for inventory in &state.economy.inventories {
        for row in &inventory.items {
            if &row.resource == item {
                add(&mut total, row.on_hand, "BAL-ore")?;
            }
        }
    }
    for inventory in &state.raw_output_inventories {
        for (resource, account) in &inventory.items {
            if resource == item {
                add(&mut total, account.on_hand, "BAL-ore")?;
            }
        }
    }
    // raw_slots is the full authoritative line. pieces is only a mirror and must not be summed.
    for belt in &state.raw_belts {
        for slot in &belt.raw_slots {
            if slot.item.as_ref() == Some(item) {
                add(&mut total, 1, "BAL-ore")?;
            }
        }
    }
    for node in &state.nodes {
        let node_id = node.content_id.clone();
        if catalog.node_item(&node_id)? == *item {
            add(&mut total, node.remaining, "BAL-ore")?;
        }
    }
    Ok(total)
}
fn signed_delta(after: u128, before: u128) -> Result<i128, CheckFailure> {
    let after =
        i128::try_from(after).map_err(|_| CheckFailure::new("BAL-ore", "stock exceeds i128"))?;
    let before =
        i128::try_from(before).map_err(|_| CheckFailure::new("BAL-ore", "stock exceeds i128"))?;
    after
        .checked_sub(before)
        .ok_or_else(|| CheckFailure::new("BAL-ore", "stock delta overflow"))
}
fn balance_expected(
    op: &OperationAudit,
    catalog: &OracleCatalog,
    item: &ContentId,
) -> Result<i128, CheckFailure> {
    let mut positive = 0_u128;
    let mut negative = 0_u128;
    for event in &op.events {
        match &event.kind {
            EconomyEventKind::ProductionStarted { charged, .. } => {
                for line in charged {
                    if &line.resource == item {
                        add(&mut negative, line.amount, "BAL-ore")?;
                    }
                }
            }
            EconomyEventKind::ProductionCancelled { refunded, .. } => {
                for line in refunded {
                    if &line.resource == item {
                        add(&mut positive, line.amount, "BAL-ore")?;
                    }
                }
            }
            EconomyEventKind::ItemsProduced { outputs, .. } => {
                for line in outputs {
                    if &line.resource == item {
                        add(&mut positive, line.amount, "BAL-ore")?;
                    }
                }
            }
            EconomyEventKind::Fired {
                weapon,
                ammo,
                item: Some(ammo_item),
                ..
            } => {
                if catalog.weapon_item(op, *weapon)?.as_ref() == Some(item) {
                    add(&mut negative, *ammo, "BAL-ore")?;
                }
                let _ = ammo_item;
            }
            EconomyEventKind::Converted {
                item: converted,
                amount,
                ..
            } if converted == item => add(&mut negative, *amount, "BAL-ore")?,
            EconomyEventKind::Destroyed { items, .. } => {
                for line in items {
                    if &line.item == item {
                        add(&mut negative, line.amount, "BAL-ore")?;
                    }
                }
            }
            _ => {}
        }
    }
    let positive = i128::try_from(positive)
        .map_err(|_| CheckFailure::new("BAL-ore", "positive event sum overflow"))?;
    let negative = i128::try_from(negative)
        .map_err(|_| CheckFailure::new("BAL-ore", "negative event sum overflow"))?;
    positive
        .checked_sub(negative)
        .ok_or_else(|| CheckFailure::new("BAL-ore", "event delta overflow"))
}
impl OracleCatalog {
    fn weapon_item(
        &self,
        op: &OperationAudit,
        weapon: u32,
    ) -> Result<Option<ContentId>, CheckFailure> {
        let unit = op
            .before
            .units
            .iter()
            .chain(&op.after.units)
            .find(|u| u.unit_id == weapon)
            .ok_or_else(|| CheckFailure::new("SHOT", format!("weapon {weapon} has no unit row")))?;
        let id = ContentId::parse(&unit.content_id)
            .map_err(|e| CheckFailure::new("SHOT", e.to_string()))?;
        let def = self.units.get(&id).ok_or_else(|| {
            CheckFailure::new("SHOT", format!("weapon {id} is absent from source RON"))
        })?;
        let item = def
            .weapon
            .as_ref()
            .and_then(|w| w.ammo.as_ref())
            .map(|a| ContentId::parse(&a.item))
            .transpose()
            .map_err(|e| CheckFailure::new("SHOT", e.to_string()))?;
        Ok(item)
    }
}
fn check_balance(
    op: &OperationAudit,
    catalog: &OracleCatalog,
    item: &ContentId,
    check: &'static str,
) -> Result<(), CheckFailure> {
    let before =
        stock(&op.before, catalog, item).map_err(|e| CheckFailure::new(check, e.detail))?;
    let after = stock(&op.after, catalog, item).map_err(|e| CheckFailure::new(check, e.detail))?;
    let actual = signed_delta(after, before).map_err(|e| CheckFailure::new(check, e.detail))?;
    let expected =
        balance_expected(op, catalog, item).map_err(|e| CheckFailure::new(check, e.detail))?;
    if actual != expected {
        return Err(CheckFailure::new(
            check,
            format!("{item}: stock delta {actual}, events require {expected}"),
        ));
    }
    Ok(())
}

pub fn check_operation(
    op: &OperationAudit,
    oracle_catalog: &OracleCatalog,
) -> Result<(), CheckFailure> {
    let ore = oracle_catalog.item(&format!("{}:ore", oracle_catalog.package))?;
    let shell = oracle_catalog.item(&format!("{}:shell", oracle_catalog.package))?;
    check_balance(op, oracle_catalog, &ore, "BAL-ore")?;
    check_balance(op, oracle_catalog, &shell, "BAL-shell")?;
    check_shots(op, oracle_catalog)?;
    check_reservations(&op.after, oracle_catalog)?;
    check_refunds(op, oracle_catalog)?;
    check_dangling(&op.after)?;
    Ok(())
}

fn check_shots(op: &OperationAudit, catalog: &OracleCatalog) -> Result<(), CheckFailure> {
    for event in &op.events {
        match &event.kind {
            EconomyEventKind::Fired {
                weapon, item, ammo, ..
            } => {
                let expected_item = catalog.weapon_item(op, *weapon)?;
                if item != &expected_item {
                    return Err(CheckFailure::new(
                        "SHOT",
                        format!(
                            "weapon {weapon}: event ammo item {item:?}, source RON says {expected_item:?}"
                        ),
                    ));
                }
                let unit = op
                    .before
                    .units
                    .iter()
                    .chain(&op.after.units)
                    .find(|u| u.unit_id == *weapon)
                    .ok_or_else(|| CheckFailure::new("SHOT", "weapon unit missing"))?;
                let content = ContentId::parse(&unit.content_id)
                    .map_err(|e| CheckFailure::new("SHOT", e.to_string()))?;
                let def = catalog
                    .units
                    .get(&content)
                    .and_then(|u| u.weapon.as_ref())
                    .ok_or_else(|| CheckFailure::new("SHOT", "weapon definition missing"))?;
                let expected = def.ammo.as_ref().map_or(0, |a| a.per_shot);
                if *ammo != expected || expected == 0 {
                    return Err(CheckFailure::new(
                        "SHOT",
                        format!(
                            "weapon {weapon}: ammo {ammo}, expected positive per_shot {expected}"
                        ),
                    ));
                }
                if let Some(ammo_id) = expected_item {
                    let delta = signed_delta(
                        stock(&op.after, catalog, &ammo_id)?,
                        stock(&op.before, catalog, &ammo_id)?,
                    )?;
                    if delta != -i128::from(expected) {
                        return Err(CheckFailure::new(
                            "SHOT",
                            format!("shot debits {delta} of {ammo_id}, expected -{expected}"),
                        ));
                    }
                }
            }
            EconomyEventKind::ItemsProduced {
                machine,
                job,
                outputs,
            } => {
                let recipe = op
                    .before
                    .raw_queues
                    .iter()
                    .chain(&op.after.raw_queues)
                    .flat_map(|q| &q.jobs)
                    .find(|j| j.id == *job)
                    .map(|j| j.recipe.clone())
                    .ok_or_else(|| {
                        CheckFailure::new(
                            "SHOT",
                            format!("produced job {job} missing from adjacent queue snapshots"),
                        )
                    })?;
                let recipe_def = catalog.recipes.get(&recipe).ok_or_else(|| {
                    CheckFailure::new("SHOT", format!("recipe {recipe} missing from source RON"))
                })?;
                let expected = recipe_def
                    .outputs
                    .iter()
                    .map(|x| Ok((ContentId::parse(&x.item)?, x.amount)))
                    .collect::<Result<Vec<_>, String>>()
                    .map_err(|e| CheckFailure::new("SHOT", e))?;
                let actual = outputs
                    .iter()
                    .map(|x| Ok((x.resource.clone(), x.amount)))
                    .collect::<Result<Vec<_>, String>>()
                    .map_err(|e| CheckFailure::new("SHOT", e))?;
                let mut expected = expected;
                let mut actual = actual;
                expected.sort_by_key(|x| x.0.clone());
                actual.sort_by_key(|x| x.0.clone());
                if actual != expected {
                    return Err(CheckFailure::new(
                        "SHOT",
                        format!("machine {machine} output differs from source recipe {recipe}"),
                    ));
                }
                for (item, amount) in expected {
                    let delta = signed_delta(
                        stock(&op.after, catalog, &item)?,
                        stock(&op.before, catalog, &item)?,
                    )?;
                    if delta != i128::from(amount) {
                        return Err(CheckFailure::new(
                            "SHOT",
                            format!(
                                "job {job} says it produced {amount} {item}, state delta is {delta}"
                            ),
                        ));
                    }
                }
            }
            EconomyEventKind::ProductionStarted {
                producer,
                job,
                charged,
            } => {
                let recipe = op
                    .before
                    .raw_queues
                    .iter()
                    .chain(&op.after.raw_queues)
                    .flat_map(|q| &q.jobs)
                    .find(|j| j.id == *job)
                    .map(|j| j.recipe.clone())
                    .ok_or_else(|| {
                        CheckFailure::new(
                            "SHOT",
                            format!("started job {job} missing from adjacent queue snapshots"),
                        )
                    })?;
                let costs = if let Some(recipe) = catalog.recipes.get(&recipe) {
                    recipe
                        .inputs
                        .iter()
                        .map(|x| Ok((ContentId::parse(&x.item)?, x.amount)))
                        .collect::<Result<Vec<_>, String>>()
                        .map_err(|e| CheckFailure::new("SHOT", e.to_string()))?
                } else if let Some(tech) = catalog.technologies.get(&recipe) {
                    tech.costs
                        .iter()
                        .filter(|x| x.source == CostSource::ProducerInventory)
                        .map(|x| Ok((ContentId::parse(&x.resource)?, x.amount)))
                        .collect::<Result<Vec<_>, String>>()
                        .map_err(|e| CheckFailure::new("SHOT", e.to_string()))?
                } else {
                    return Err(CheckFailure::new(
                        "SHOT",
                        format!("started content missing from source RON: {recipe}"),
                    ));
                };
                let mut expected = costs;
                let mut actual = charged
                    .iter()
                    .map(|x| Ok((x.resource.clone(), x.amount)))
                    .collect::<Result<Vec<_>, String>>()
                    .map_err(|e| CheckFailure::new("SHOT", e))?;
                expected.sort_by_key(|x| x.0.clone());
                actual.sort_by_key(|x| x.0.clone());
                if actual != expected {
                    return Err(CheckFailure::new(
                        "SHOT",
                        format!("producer {producer} charge differs from source RON"),
                    ));
                }
                for (item, amount) in expected {
                    let delta = signed_delta(
                        stock(&op.after, catalog, &item)?,
                        stock(&op.before, catalog, &item)?,
                    )?;
                    if delta != -i128::from(amount) {
                        return Err(CheckFailure::new(
                            "SHOT",
                            format!("job {job} charged {amount} {item}, stock delta {delta}"),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn check_reservations(state: &AuditState, catalog: &OracleCatalog) -> Result<(), CheckFailure> {
    let mut totals: BTreeMap<((u8, u32), ContentId), u128> = BTreeMap::new();
    for claim in &state.ledger_claims {
        if let ReservationClaimView::Active {
            map_job_id,
            token_job_id,
            lines,
        } = claim
        {
            let exists = *map_job_id == *token_job_id
                && state
                    .economy
                    .queues
                    .iter()
                    .flat_map(|q| &q.jobs)
                    .any(|j| j.id == *map_job_id && j.stage == JobStage::Queued);
            if !exists {
                return Err(CheckFailure::new(
                    "RES",
                    format!("active claim {map_job_id} does not name exactly one queued job"),
                ));
            }
            for line in lines {
                let endpoint = match line.endpoint {
                    ReservationEndpointView::Slot(slot) => (0, u32::from(slot)),
                    ReservationEndpointView::Inventory(unit) => (1, unit),
                    ReservationEndpointView::Output(unit) => (2, unit),
                };
                let total = totals.entry((endpoint, line.resource.clone())).or_default();
                *total = total
                    .checked_add(u128::from(line.amount))
                    .ok_or_else(|| CheckFailure::new("RES", "claim sum overflow"))?;
            }
        }
    }
    for queue in &state.raw_queues {
        for job in &queue.jobs {
            let reserved_technology = catalog.technologies.get(&job.recipe).is_some_and(|tech| {
                tech.payment == turts_mod_api::PaymentPolicy::ReserveThenCharge
            });
            let active = state.ledger_claims.iter().filter(|claim| matches!(claim, ReservationClaimView::Active { map_job_id, .. } if *map_job_id == job.id)).count();
            if reserved_technology && job.stage == JobStage::Queued && active != 1 {
                return Err(CheckFailure::new(
                    "RES",
                    format!(
                        "queued reserve-then-charge job {} has {active} active claims",
                        job.id
                    ),
                ));
            }
            if job.stage != JobStage::Queued && active != 0 {
                return Err(CheckFailure::new(
                    "RES",
                    format!("started job {} still has an active claim", job.id),
                ));
            }
        }
    }
    for balance in &state.economy.balances {
        if balance.reserved > balance.on_hand {
            return Err(CheckFailure::new(
                "RES",
                format!(
                    "slot {} {} reserved above on_hand",
                    balance.slot, balance.resource
                ),
            ));
        }
        let claimed = totals
            .get(&((0, u32::from(balance.slot)), balance.resource.clone()))
            .copied()
            .unwrap_or(0);
        if claimed != u128::from(balance.reserved) {
            return Err(CheckFailure::new(
                "RES",
                format!("slot reservation ledger differs for {}", balance.resource),
            ));
        }
    }
    for inventory in &state.economy.inventories {
        let mut used = 0_u128;
        for item in &inventory.items {
            if item.reserved > item.on_hand {
                return Err(CheckFailure::new(
                    "RES",
                    format!(
                        "inventory {} {} reserved above on_hand",
                        inventory.unit, item.resource
                    ),
                ));
            }
            used = used
                .checked_add(u128::from(item.on_hand))
                .ok_or_else(|| CheckFailure::new("RES", "inventory sum overflow"))?;
            let claimed = totals
                .get(&((1, inventory.unit), item.resource.clone()))
                .copied()
                .unwrap_or(0);
            if claimed != u128::from(item.reserved) {
                return Err(CheckFailure::new(
                    "RES",
                    format!("inventory reservation ledger differs for {}", item.resource),
                ));
            }
        }
        if used > u128::from(inventory.capacity) {
            return Err(CheckFailure::new(
                "RES",
                format!(
                    "inventory {} uses {used} > {}",
                    inventory.unit, inventory.capacity
                ),
            ));
        }
    }
    for ((endpoint, item), claimed) in &totals {
        let reserved = match endpoint.0 {
            0 => state
                .economy
                .balances
                .iter()
                .find(|row| u32::from(row.slot) == endpoint.1 && &row.resource == item)
                .map(|row| row.reserved),
            1 => state
                .economy
                .inventories
                .iter()
                .find(|row| row.unit == endpoint.1)
                .and_then(|row| row.items.iter().find(|entry| &entry.resource == item))
                .map(|row| row.reserved),
            2 => state
                .raw_output_inventories
                .iter()
                .find(|row| row.unit == endpoint.1)
                .and_then(|row| row.items.iter().find(|entry| &entry.0 == item))
                .map(|row| row.1.reserved),
            _ => None,
        }
        .ok_or_else(|| {
            CheckFailure::new(
                "RES",
                format!(
                    "active claim points to missing endpoint {:?} for {item}",
                    endpoint
                ),
            )
        })?;
        if u128::from(reserved) != *claimed {
            return Err(CheckFailure::new(
                "RES",
                format!(
                    "active claims total {claimed} but endpoint {:?} reserves {reserved} of {item}",
                    endpoint
                ),
            ));
        }
    }
    for output in &state.raw_output_inventories {
        let mut used = 0_u128;
        for (item, account) in &output.items {
            if account.reserved > account.on_hand {
                return Err(CheckFailure::new(
                    "RES",
                    format!("output {} reserved above on_hand", output.unit),
                ));
            }
            used = used
                .checked_add(u128::from(account.on_hand))
                .ok_or_else(|| CheckFailure::new("RES", "output sum overflow"))?;
            let claimed = totals
                .get(&((2, output.unit), item.clone()))
                .copied()
                .unwrap_or(0);
            if claimed != u128::from(account.reserved) {
                return Err(CheckFailure::new(
                    "RES",
                    format!("output reservation ledger differs for {}", item),
                ));
            }
        }
        if used > u128::from(output.capacity) {
            return Err(CheckFailure::new(
                "RES",
                format!("output {} uses {used} > {}", output.unit, output.capacity),
            ));
        }
    }
    Ok(())
}
pub fn check_refunds(op: &OperationAudit, catalog: &OracleCatalog) -> Result<(), CheckFailure> {
    for event in &op.events {
        let EconomyEventKind::ProductionCancelled {
            producer,
            job,
            refunded,
            ..
        } = &event.kind
        else {
            continue;
        };
        let row = op
            .before
            .raw_queues
            .iter()
            .find(|q| q.producer == *producer)
            .and_then(|q| q.jobs.iter().find(|j| j.id == *job))
            .ok_or_else(|| {
                CheckFailure::new("REFUND", format!("cancelled job {job} missing before"))
            })?;
        let ratio: (u32, u32) = if let Some(recipe) = catalog.recipes.get(&row.recipe) {
            match row.stage {
                JobStage::Queued => (1, 1),
                JobStage::Running => (
                    recipe.cancel_refunds.running.numerator,
                    recipe.cancel_refunds.running.denominator,
                ),
                JobStage::OutputBlocked => (
                    recipe.cancel_refunds.output_blocked.numerator,
                    recipe.cancel_refunds.output_blocked.denominator,
                ),
            }
        } else if let Some(tech) = catalog.technologies.get(&row.recipe) {
            let RefundRatioFor {
                numerator,
                denominator,
            } = refund_ratio(tech.cancel_refunds, row.stage);
            (numerator, denominator)
        } else {
            return Err(CheckFailure::new(
                "REFUND",
                format!("unknown cancelled content {}", row.recipe),
            ));
        };
        if ratio.1 == 0 {
            return Err(CheckFailure::new("REFUND", "zero refund denominator"));
        }
        let actual: BTreeMap<_, _> = refunded
            .iter()
            .map(|line| (line.resource.clone(), line.amount))
            .collect();
        let mut expected_refunds = BTreeMap::new();
        for charged in &row.charged {
            let product = u128::from(charged.amount)
                .checked_mul(u128::from(ratio.0))
                .ok_or_else(|| CheckFailure::new("REFUND", "refund multiplication overflow"))?;
            let expected = u64::try_from(product / u128::from(ratio.1))
                .map_err(|_| CheckFailure::new("REFUND", "refund conversion overflow"))?;
            let total = expected_refunds
                .entry(charged.resource.clone())
                .or_insert(0_u64);
            *total = total
                .checked_add(expected)
                .ok_or_else(|| CheckFailure::new("REFUND", "refund total overflow"))?;
        }
        if actual != expected_refunds {
            return Err(CheckFailure::new(
                "REFUND",
                format!("job {job} refunds {actual:?}, expected {expected_refunds:?}"),
            ));
        }
    }
    Ok(())
}
#[derive(Debug, Clone, Copy)]
struct RefundRatioFor {
    numerator: u32,
    denominator: u32,
}
fn refund_ratio(refunds: CancelRefunds, stage: JobStage) -> RefundRatioFor {
    let ratio = match stage {
        JobStage::Queued => refunds.queued,
        JobStage::Running => refunds.running,
        JobStage::OutputBlocked => refunds.output_blocked,
    };
    RefundRatioFor {
        numerator: ratio.numerator,
        denominator: ratio.denominator,
    }
}
pub fn check_dangling(state: &AuditState) -> Result<(), CheckFailure> {
    for id in &state.removed_units {
        if state.raw_units.iter().any(|r| r.unit_id == *id)
            || state.units.iter().any(|r| r.unit_id == *id)
            || state.economy.inventories.iter().any(|r| r.unit == *id)
            || state.raw_output_inventories.iter().any(|r| r.unit == *id)
            || state
                .raw_inserters
                .iter()
                .any(|r| r.unit == *id || r.source == Some(*id) || r.destination == Some(*id))
            || state.raw_extractors.iter().any(|r| r.unit == *id)
            || state.raw_weapons.iter().any(|r| r.unit == *id)
            || state.raw_healths.iter().any(|r| r.unit == *id)
            || state.disabled.contains(id)
            || state
                .raw_belts
                .iter()
                .any(|r| r.first_piece == *id || r.pieces.iter().any(|p| p.unit == *id))
            || state
                .raw_orders
                .iter()
                .any(|r| r.unit == *id || r.receiver == Some(*id))
            || state.raw_queues.iter().any(|r| r.producer == *id)
        {
            return Err(CheckFailure::new(
                "DANGLING",
                format!("removed unit {id} remains in authoritative state"),
            ));
        }
        if state.ledger_claims.iter().any(|claim| matches!(claim, ReservationClaimView::Active { lines, .. } if lines.iter().any(|line| matches!(line.endpoint, ReservationEndpointView::Inventory(unit) | ReservationEndpointView::Output(unit) if unit == *id)))) {
            return Err(CheckFailure::new("DANGLING", format!("removed unit {id} remains in reservation claim")));
        }
    }
    Ok(())
}

pub fn check_tick(
    ops: &[OperationAudit],
    history: &mut OracleHistory,
    catalog: &OracleCatalog,
) -> Result<(), CheckFailure> {
    let completed_now: BTreeMap<u8, BTreeSet<ContentId>> = ops
        .iter()
        .flat_map(|op| &op.events)
        .filter_map(|event| match &event.kind {
            EconomyEventKind::ResearchCompleted {
                slot, technology, ..
            } => Some((*slot, technology.clone())),
            _ => None,
        })
        .fold(BTreeMap::new(), |mut acc, (slot, tech)| {
            acc.entry(slot).or_default().insert(tech);
            acc
        });
    let mut effective_completed = history.completed_by_slot.clone();
    let killed_this_tick: BTreeSet<u32> = ops
        .iter()
        .flat_map(|op| &op.events)
        .filter_map(|event| match event.kind {
            EconomyEventKind::Destroyed {
                unit,
                cause: turts_sim::economy::events::RemoveCause::Killed { .. },
                ..
            } => Some(unit),
            _ => None,
        })
        .collect();
    for (slot, technologies) in &completed_now {
        effective_completed
            .entry(*slot)
            .or_default()
            .extend(technologies.iter().cloned());
    }
    for op in ops {
        for event in &op.events {
            match &event.kind {
                EconomyEventKind::ResearchCompleted { .. } => {}
                EconomyEventKind::ProductionStarted { job, .. } => {
                    if !history.started_jobs.insert(*job) {
                        return Err(CheckFailure::new(
                            "SHOT",
                            format!("job {job} has more than one ProductionStarted event"),
                        ));
                    }
                }
                EconomyEventKind::Destroyed {
                    unit,
                    cause: turts_sim::economy::events::RemoveCause::Killed { .. },
                    ..
                } => {
                    if !history.destroyed_events.insert(*unit) {
                        return Err(CheckFailure::new(
                            "DMG",
                            format!("unit {unit} has more than one Killed event"),
                        ));
                    }
                    let final_health = op
                        .after
                        .raw_healths
                        .iter()
                        .find(|row| row.unit == *unit)
                        .map(|row| row.current)
                        .or_else(|| history.health.get(unit).copied());
                    if final_health != Some(0) || !op.after.removed_units.contains(unit) {
                        return Err(CheckFailure::new(
                            "DMG",
                            format!(
                                "Killed event for {unit} is not paired with zero health and removal"
                            ),
                        ));
                    }
                    history.destroyed.insert(*unit);
                }
                EconomyEventKind::Fired {
                    weapon,
                    target,
                    damage,
                    applied,
                    ..
                } => {
                    let source = op
                        .before
                        .units
                        .iter()
                        .chain(&op.after.units)
                        .find(|u| u.unit_id == *weapon)
                        .ok_or_else(|| {
                            CheckFailure::new("DMG", format!("weapon {weapon} has no unit row"))
                        })?;
                    let shooter = ContentId::parse(&source.content_id)
                        .map_err(|e| CheckFailure::new("DMG", e.to_string()))?;
                    let def = catalog
                        .units
                        .get(&shooter)
                        .and_then(|u| u.weapon.as_ref())
                        .ok_or_else(|| {
                            CheckFailure::new(
                                "DMG",
                                format!("weapon {shooter} missing from source RON"),
                            )
                        })?;
                    let mut expected_damage = u128::from(def.damage);
                    let researched = effective_completed
                        .get(&source.owner_slot)
                        .into_iter()
                        .flatten();
                    for tech_id in researched {
                        if let Some(tech) = catalog.technologies.get(tech_id) {
                            for effect in &tech.effects {
                                let turts_mod_api::EffectDef::WeaponModifier {
                                    unit,
                                    stat: turts_mod_api::WeaponStat::Damage,
                                    add,
                                } = effect;
                                if unit == &shooter.to_string() {
                                    expected_damage =
                                        expected_damage.checked_add(u128::from(*add)).ok_or_else(
                                            || CheckFailure::new("DMG", "damage overflow"),
                                        )?;
                                }
                            }
                        }
                    }
                    let expected_damage = u64::try_from(expected_damage)
                        .map_err(|_| CheckFailure::new("DMG", "damage does not fit u64"))?;
                    if *damage != expected_damage {
                        return Err(CheckFailure::new(
                            "DMG",
                            format!(
                                "weapon {weapon} fired {damage}, expected {expected_damage} from research completed by tick end"
                            ),
                        ));
                    }
                    let before = op
                        .before
                        .raw_healths
                        .iter()
                        .find(|h| h.unit == *target)
                        .map(|h| h.current)
                        .or_else(|| history.health.get(target).copied())
                        .ok_or_else(|| {
                            CheckFailure::new("DMG", format!("target {target} has no health row"))
                        })?;
                    let expected_applied = before.min(expected_damage);
                    if *applied != expected_applied {
                        return Err(CheckFailure::new(
                            "DMG",
                            format!(
                                "target {target} applied {applied}, expected min({before},{expected_damage})"
                            ),
                        ));
                    }
                    let after = op
                        .after
                        .raw_healths
                        .iter()
                        .find(|h| h.unit == *target)
                        .map(|h| h.current)
                        .unwrap_or(0);
                    if before.checked_sub(after) != Some(*applied) {
                        return Err(CheckFailure::new(
                            "DMG",
                            format!("target {target} health delta does not equal applied damage"),
                        ));
                    }
                    if history.destroyed.contains(target) || killed_this_tick.contains(target) && ops.iter().any(|later| later.operation_id > op.operation_id && later.events.iter().any(|e| matches!(e.kind, EconomyEventKind::Fired { target: later_target, .. } if later_target == *target))) { return Err(CheckFailure::new("DMG", format!("fired at destroyed unit {target}"))); }
                    history.health.insert(*target, after);
                }
                _ => {}
            }
        }
        history
            .destroyed
            .extend(op.after.removed_units.iter().copied());
    }
    for (slot, technologies) in completed_now {
        history
            .completed_by_slot
            .entry(slot)
            .or_default()
            .extend(technologies);
    }
    for (unit, hp) in &history.health {
        if *hp == 0 && !history.destroyed_events.contains(unit) {
            return Err(CheckFailure::new(
                "DMG",
                format!("unit {unit} reached zero health without exactly one Killed event"),
            ));
        }
    }
    if let Some(last) = ops.last() {
        for (unit, hp) in &history.health {
            if let Some(actual) = last.after.raw_healths.iter().find(|r| r.unit == *unit)
                && actual.current != *hp
            {
                return Err(CheckFailure::new(
                    "DMG",
                    format!("unit {unit} audit health {}, expected {hp}", actual.current),
                ));
            }
        }
    }
    Ok(())
}
