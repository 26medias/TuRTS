//! Effective armour from immutable definitions and completed technologies in completion order.
use crate::{ContentCatalog, ContentId, ResolvedEffect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmourOverflow;

/// A new per-type row captures the current flat value. Later Flat upgrades do not change it.
pub fn armour_for<'a>(
    catalog: &ContentCatalog,
    unit: &ContentId,
    completed_order: impl IntoIterator<Item = &'a ContentId>,
) -> Result<Option<crate::ArmourDef>, ArmourOverflow> {
    let Some(mut armour) = catalog.units.get(unit).and_then(|def| def.armour.clone()) else {
        return Ok(None);
    };
    for technology in completed_order {
        let Some(definition) = catalog.technologies.get(technology) else {
            continue;
        };
        for effect in &definition.effects {
            let ResolvedEffect::ArmourModifier {
                unit: target,
                stat,
                add,
            } = effect
            else {
                continue;
            };
            if target != unit {
                continue;
            }
            match stat {
                crate::ArmourStat::Flat => {
                    armour.flat = armour.flat.checked_add(*add).ok_or(ArmourOverflow)?
                }
                crate::ArmourStat::Points => {
                    armour.points = armour.points.checked_add(*add).ok_or(ArmourOverflow)?
                }
                crate::ArmourStat::FlatByType(damage) => {
                    if let Some((_, value)) = armour
                        .flat_by_type
                        .iter_mut()
                        .find(|(kind, _)| kind == damage)
                    {
                        *value = value.checked_add(*add).ok_or(ArmourOverflow)?;
                    } else {
                        armour.flat_by_type.push((
                            damage.clone(),
                            armour.flat.checked_add(*add).ok_or(ArmourOverflow)?,
                        ));
                    }
                }
            }
        }
    }
    armour
        .flat_by_type
        .sort_by(|left, right| left.0.cmp(&right.0));
    Ok(Some(armour))
}
