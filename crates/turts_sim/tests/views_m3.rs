use serde::{Deserialize, Serialize};
use turts_mod_api::ContentId;
use turts_sim::{
    RejectReason,
    economy::{
        ledger::Account,
        views_m3::{
            CatalogEffectAudit, CatalogItemRecipeAudit, CatalogItemStackAudit,
            CatalogTechnologyAudit, M3AuditChanges, M3AuditState, RawBeltAudit, RawBeltPieceAudit,
            RawBeltSlotAudit, RawEnabledAudit, RawExtractorAudit, RawExtractorStatusAudit,
            RawHealthAudit, RawInserterAudit, RawInserterStageAudit, RawInserterWaitAudit,
            RawOutputInventoryAudit, RawResearchAudit, RawWeaponAudit, RawWeaponStatusAudit,
        },
    },
};

fn is_public_serde<T: Serialize + for<'de> Deserialize<'de>>() {}

#[test]
fn views_expose_no_pub_crate_types() {
    is_public_serde::<RawOutputInventoryAudit>();
    is_public_serde::<RawBeltSlotAudit>();
    is_public_serde::<RawBeltPieceAudit>();
    is_public_serde::<RawBeltAudit>();
    is_public_serde::<RawInserterWaitAudit>();
    is_public_serde::<RawInserterStageAudit>();
    is_public_serde::<RawInserterAudit>();
    is_public_serde::<RawExtractorStatusAudit>();
    is_public_serde::<RawExtractorAudit>();
    is_public_serde::<RawWeaponStatusAudit>();
    is_public_serde::<RawWeaponAudit>();
    is_public_serde::<RawHealthAudit>();
    is_public_serde::<RawResearchAudit>();
    is_public_serde::<RawEnabledAudit>();
    is_public_serde::<M3AuditState>();
    is_public_serde::<M3AuditChanges>();
    is_public_serde::<CatalogTechnologyAudit>();
    is_public_serde::<CatalogEffectAudit>();
    is_public_serde::<CatalogItemRecipeAudit>();
    is_public_serde::<CatalogItemStackAudit>();

    let item = ContentId::parse("fixture:item").unwrap();
    let output = RawOutputInventoryAudit {
        unit: 2,
        capacity: 10,
        accepted_items: vec![item.clone()],
        items: vec![(
            item.clone(),
            Account {
                on_hand: 1,
                reserved: 0,
            },
        )],
    };
    assert_eq!(output.items[0].1.on_hand, 1);

    let belt = RawBeltAudit {
        first_piece: 3,
        definition: ContentId::parse("fixture:belt").unwrap(),
        owner_slot: 1,
        slots_per_piece: 1,
        slots_per_second: 20,
        rate_carry: 0,
        raw_slots: vec![RawBeltSlotAudit {
            item_index: 1,
            item: Some(item.clone()),
        }],
        pieces: vec![RawBeltPieceAudit {
            unit: 3,
            slots: vec![RawBeltSlotAudit {
                item_index: 1,
                item: Some(item.clone()),
            }],
        }],
    };
    assert_eq!(belt.raw_slots[0].item.as_ref(), Some(&item));
    assert_eq!(belt.pieces[0].slots[0].item.as_ref(), Some(&item));

    let inserter = RawInserterAudit {
        unit: 4,
        enabled: true,
        stage: RawInserterStageAudit::AtSource(Some(RawInserterWaitAudit::SourceEmpty)),
        elapsed: 2,
        source: Some(2),
        destination: Some(8),
    };
    assert_eq!(inserter.unit, 4);
    let extractor = RawExtractorAudit {
        unit: 6,
        enabled: true,
        node: Some(9),
        rate_carry: 3,
        status: RawExtractorStatusAudit::Extracting,
    };
    assert_eq!(extractor.node, Some(9));
    let weapon = RawWeaponAudit {
        unit: 7,
        enabled: true,
        cooldown: 0,
        effective_damage: Err(RejectReason::ArithmeticOverflow),
        status: RawWeaponStatusAudit::Ready,
        last_target: None,
    };
    assert_eq!(
        weapon.effective_damage,
        Err(RejectReason::ArithmeticOverflow)
    );
    let research = RawResearchAudit {
        slot: 1,
        technology: ContentId::parse("fixture:tech").unwrap(),
    };
    assert_eq!(research.slot, 1);
}

#[test]
fn equal_value_write_makes_no_change_record() {
    use turts_sim::economy::views_m3::{M3AuditState, m3_changes};
    let mut before = M3AuditState::default();
    before.raw_healths.push(RawHealthAudit {
        unit: 5,
        current: 20,
    });
    let after = before.clone();
    let changes = m3_changes(&before, &after);
    assert!(changes.healths.is_empty());
}
