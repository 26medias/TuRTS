//! Compatibility selection for the former computer-slot option. There is no private strategy.
use crate::config::{BotConfig, Cli, HostConfig};
use turts_mod_api::ContentCatalog;

pub(crate) fn configuration(
    config: &HostConfig,
    cli: &Cli,
    catalog: &ContentCatalog,
    root_game: &str,
) -> Result<Vec<BotConfig>, String> {
    if let Some(slot) = cli.computer_slot {
        if !config.bots.is_empty() || !cli.bots.is_empty() {
            return Err("--computer-slot cannot be combined with explicit bot profiles".into());
        }
        let exported = catalog
            .bots
            .keys()
            .filter(|id| id.package.0 == root_game)
            .collect::<Vec<_>>();
        if exported.len() != 1 {
            return Err("--computer-slot needs exactly one exported public bot profile; use --bot SLOT,PROFILE,SEED".into());
        }
        let Some(profile) = exported.first() else {
            return Err("--computer-slot has no exported public bot profile".into());
        };
        return Ok(vec![BotConfig {
            slot,
            profile: profile.to_string(),
            seed: 0,
        }]);
    }
    let mut selected = config.bots.clone();
    selected.extend(cli.bots.iter().cloned());
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compatibility_alias_selects_only_the_root_export_not_imported_profiles() {
        use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
        let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
        let mut catalog = archive.content().unwrap();
        let mut imported = catalog.bots.values().next().unwrap().clone();
        imported.id = "custom".into();
        catalog.bots.insert(
            turts_mod_api::ContentId::parse("library:custom").unwrap(),
            imported,
        );
        let cfg=HostConfig::from_ron(r#"(game_package:"game.tar",http_listen:"127.0.0.1:1",ws_listen:"127.0.0.1:2",public_url:"http://127.0.0.1:1",tick_hz:20)"#).unwrap();
        let cli =
            crate::config::parse_cli(&["server.ron", "--computer-slot", "1"].map(str::to_owned))
                .unwrap();
        let selected = configuration(&cfg, &cli, &catalog, "stockyard").unwrap();
        assert_eq!(selected[0].profile, "stockyard:opponent");
        assert!(configuration(&cfg, &cli, &catalog, "other_game").is_err());
    }
}
