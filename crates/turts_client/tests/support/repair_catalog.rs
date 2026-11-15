use std::{path::Path, sync::OnceLock};
use turts_mod_api::{
    ContentCatalog, ContentId, RepairCost, RepairerDef, SlotRelation, TargetFilter,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

pub fn catalog() -> ContentCatalog {
    static CATALOG: OnceLock<ContentCatalog> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
            let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
            let mut catalog = archive.content().unwrap();
            let mut engineer = catalog.units[&ContentId::parse("skirmish:worker").unwrap()].clone();
            engineer.id = ContentId::parse("skirmish:engineer").unwrap();
            engineer.name = "Engineer".into();
            engineer.gatherer = None;
            engineer.inventory = None;
            engineer.constructor = catalog.units[&ContentId::parse("skirmish:base").unwrap()]
                .constructor
                .clone();
            engineer.build_area = Some(16.0);
            engineer.placement =
                catalog.units[&ContentId::parse("skirmish:barracks").unwrap()].placement;
            engineer.repairer = Some(RepairerDef {
                rate_per_second: 1,
                reach: 0.5,
                cost: vec![RepairCost {
                    resource: "skirmish:credits".into(),
                    amount: 1,
                    per_health: 3,
                }],
                targets: TargetFilter {
                    slots: SlotRelation::Same,
                },
                classes: vec![],
                auto_range: None,
            });
            catalog.units.insert(engineer.id.clone(), engineer);
            catalog
        })
        .clone()
}
