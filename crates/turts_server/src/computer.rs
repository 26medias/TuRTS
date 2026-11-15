//! Compatibility selection for the former computer-slot option. There is no private strategy.
use crate::config::{BotConfig, Cli, HostConfig};
use turts_mod_api::ContentCatalog;

pub(crate) fn configuration(
    config: &HostConfig,
    cli: &Cli,
    catalog: &ContentCatalog,
) -> Result<Vec<BotConfig>, String> {
    if let Some(slot) = cli.computer_slot {
        if !config.bots.is_empty() || !cli.bots.is_empty() {
            return Err("--computer-slot cannot be combined with explicit bot profiles".into());
        }
        if catalog.bots.len() != 1 {
            return Err("--computer-slot needs exactly one exported public bot profile; use --bot SLOT,PROFILE,SEED".into());
        }
        let Some(profile) = catalog.bots.keys().next() else {
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
