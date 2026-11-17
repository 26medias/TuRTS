//! Validate the portable operating-resource contract, including merged catalogs.

use std::collections::BTreeSet;
use turts_mod_api::{
    ContentCatalog, ContentId, Manifest, ModError, ResourceKind, ShortEffect, SupplyCounts,
    SupplyScope,
};

pub(crate) fn validate(manifest: &Manifest, catalog: &ContentCatalog) -> Result<(), ModError> {
    let mut scale = 1_u128;
    for unit in catalog.units.values() {
        let file = catalog
            .source_files
            .get(&unit.id)
            .map_or("content", String::as_str);
        let fail = |path: &str, message: String| {
            ModError::new(&manifest.id, path, message).with_file(file)
        };
        let resource = |path: &str, name: &str, supply: bool| -> Result<(), ModError> {
            let id = ContentId::parse(name).map_err(|error| fail(path, error.to_string()))?;
            let Some(definition) = catalog.resources.get(&id) else {
                return Err(fail(path, format!("unknown resource `{name}`")));
            };
            let expected = if supply {
                ResourceKind::Supply {
                    scope: SupplyScope::Slot,
                    counts: SupplyCounts::Completed,
                }
            } else {
                ResourceKind::Balance
            };
            // API 0.2.18: a Network supply resource passes here. `install_utilities` checks
            // the utility type, the port and the owner of each use.
            let network = ResourceKind::Supply {
                scope: SupplyScope::Network,
                counts: SupplyCounts::Completed,
            };
            if definition.kind != expected && !(supply && definition.kind == network) {
                return Err(fail(
                    path,
                    format!("`{name}` must be a {expected:?} resource"),
                ));
            }
            Ok(())
        };
        for (field, rows) in [("provides", &unit.provides), ("consumes", &unit.consumes)] {
            let mut seen = BTreeSet::new();
            for (index, row) in rows.iter().enumerate() {
                let path = format!("{field}[{index}]");
                resource(&format!("{path}.resource"), &row.resource, true)?;
                if row.amount == 0 {
                    return Err(fail(&format!("{path}.amount"), "must be positive".into()));
                }
                if !seen.insert(&row.resource) {
                    return Err(fail(
                        &format!("{path}.resource"),
                        format!("second entry for `{}`", row.resource),
                    ));
                }
            }
        }
        if let Some(income) = &unit.income {
            resource("income.resource", &income.resource, false)?;
            if income.amount == 0 || income.period_ms == 0 {
                return Err(fail(
                    "income",
                    "amount and period_ms must be positive".into(),
                ));
            }
            if unit.health.is_none() {
                return Err(fail("income", "a unit with income needs health".into()));
            }
        }
        let lists = [
            unit.producer
                .as_ref()
                .map(|p| ("producer", p.when_short.as_slice())),
            unit.constructor
                .as_ref()
                .map(|c| ("constructor", c.when_short.as_slice())),
            unit.weapon
                .as_ref()
                .map(|w| ("weapon", w.when_short.as_slice())),
        ];
        for (owner, rows) in lists.into_iter().flatten() {
            let mut seen = BTreeSet::new();
            for (index, row) in rows.iter().enumerate() {
                let path = format!("{owner}.when_short[{index}]");
                resource(&format!("{path}.resource"), &row.resource, true)?;
                if !seen.insert(&row.resource) {
                    return Err(fail(
                        &format!("{path}.resource"),
                        format!("second entry for `{}`", row.resource),
                    ));
                }
                match row.effect {
                    ShortEffect::Rate(ratio) if owner != "weapon" => {
                        if ratio.numerator == 0
                            || ratio.denominator == 0
                            || ratio.numerator > ratio.denominator
                        {
                            return Err(fail(
                                &format!("{path}.effect"),
                                "Rate needs 0 < numerator <= denominator".into(),
                            ));
                        }
                        let denominator = u128::from(ratio.denominator);
                        let (mut a, mut b) = (scale, denominator);
                        while b != 0 {
                            (a, b) = (b, a % b);
                        }
                        scale = scale / a * denominator;
                        if scale > 65_536 {
                            return Err(fail(
                                &format!("{path}.effect"),
                                format!(
                                    "the merged Rate denominator scale is {scale}; maximum is 65536"
                                ),
                            ));
                        }
                    }
                    ShortEffect::Disable if owner == "weapon" => {}
                    ShortEffect::Rate(_) => {
                        return Err(fail(
                            &format!("{path}.effect"),
                            "Rate applies to producer and constructor only".into(),
                        ));
                    }
                    ShortEffect::Disable => {
                        return Err(fail(
                            &format!("{path}.effect"),
                            "Disable applies to weapon only".into(),
                        ));
                    }
                    ShortEffect::BlockStart => {
                        return Err(fail(
                            &format!("{path}.effect"),
                            "BlockStart is reserved for a later engine API".into(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
