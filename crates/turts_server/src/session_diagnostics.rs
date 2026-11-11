//! Private, run-local session observations for browser acceptance evidence.
//!
//! This file is enabled only with the explicit local `--stats PATH` test hook. It is written
//! beside PATH as `<PATH filename>.sessions.jsonl`, outside the served static tree. Raw player
//! IDs appear only in the mode-0600 alias records; ordinary lifecycle records use opaque aliases.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use turts_sim::{SlotId, Slots, Tick};

use crate::session::Session;
use crate::stats::MatchId;

fn exact_u64_decimal(value: u64) -> String {
    value.to_string()
}

#[derive(Resource)]
pub struct SessionDiagnostics {
    file: File,
    run_nonce: String,
    started: Instant,
    sequence: u64,
    failed: Option<String>,
    last_wall_us: Option<u128>,
    last_mono_us: Option<u128>,
    next_alias: u64,
    aliases: BTreeMap<u64, u64>,
    next_epoch: u64,
    epochs: BTreeMap<u64, ConnectionEpoch>,
}

#[derive(Clone, Copy)]
struct ConnectionEpoch {
    epoch: u64,
    alias: u64,
    slot: Option<u8>,
    player_id: u64,
}

impl SessionDiagnostics {
    pub fn sibling_path(stats_path: &Path) -> PathBuf {
        let name = stats_path.file_name().unwrap_or_default().to_string_lossy();
        stats_path.with_file_name(format!("{name}.sessions.jsonl"))
    }

    pub fn open(stats_path: &Path) -> std::io::Result<Self> {
        let path = Self::sibling_path(stats_path);
        let exists = match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if metadata.permissions().mode() & 0o777 != 0o600 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "existing private session observer must already have mode 0600",
                        ));
                    }
                }
                true
            }
            Ok(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "private session observer path must be a regular file",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error),
        };
        let file = if exists {
            let mut options = OpenOptions::new();
            options.append(true).write(true);
            options.open(&path)?
        } else {
            let mut options = OpenOptions::new();
            options.create_new(true).append(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
                options.mode(0o600);
                let file = options.open(&path)?;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
                file
            }
            #[cfg(not(unix))]
            options.open(&path)?
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                std::io::Error::other(format!("system clock precedes Unix epoch: {error}"))
            })?
            .as_nanos();
        let mut diagnostics = Self::with_file(file, now);
        diagnostics.write_record(serde_json::json!({"event":"boot_start"}))?;
        Ok(diagnostics)
    }

    fn with_file(file: File, now: u128) -> Self {
        Self {
            file,
            run_nonce: format!("{}-{now:x}", std::process::id()),
            started: Instant::now(),
            sequence: 0,
            failed: None,
            last_wall_us: None,
            last_mono_us: None,
            next_alias: 0,
            aliases: BTreeMap::new(),
            next_epoch: 0,
            epochs: BTreeMap::new(),
        }
    }

    fn alias(&mut self, player_id: u64) -> u64 {
        if let Some(alias) = self.aliases.get(&player_id) {
            return *alias;
        }
        self.next_alias = self
            .next_alias
            .checked_add(1)
            .expect("private observer alias overflow");
        let alias = self.next_alias;
        self.aliases.insert(player_id, alias);
        self.must_write(serde_json::json!({
            "event": "identity_alias",
            "identity_alias": alias,
            "player_id_decimal": player_id.to_string(),
        }));
        alias
    }

    fn epoch(&mut self, entity: Entity, player_id: u64, slot: Option<SlotId>) -> ConnectionEpoch {
        let key = entity.to_bits();
        if let Some(existing) = self
            .epochs
            .get(&key)
            .copied()
            .filter(|existing| existing.player_id == player_id)
        {
            if existing.slot == slot.map(|value| value.0) {
                return existing;
            }
            let updated = ConnectionEpoch {
                slot: slot.map(|value| value.0),
                ..existing
            };
            self.epochs.insert(key, updated);
            return updated;
        }
        self.next_epoch = self
            .next_epoch
            .checked_add(1)
            .expect("private observer epoch overflow");
        let epoch = ConnectionEpoch {
            epoch: self.next_epoch,
            alias: self.alias(player_id),
            slot: slot.map(|value| value.0),
            player_id,
        };
        self.epochs.insert(key, epoch);
        epoch
    }

    fn owner_snapshot(&mut self, owners: &[Option<u64>]) -> Vec<serde_json::Value> {
        owners
            .iter()
            .enumerate()
            .map(|(slot, owner)| {
                serde_json::json!({
                    "slot": slot,
                    "identity_alias": owner.map(|player_id| self.alias(player_id)),
                })
            })
            .collect()
    }

    fn must_write(&mut self, value: serde_json::Value) {
        if let Err(error) = self.write_record(value) {
            panic!("private session observer failed: {error}");
        }
    }

    fn write_record(&mut self, value: serde_json::Value) -> std::io::Result<()> {
        if let Some(failure) = &self.failed {
            return Err(std::io::Error::other(format!(
                "private session observer already failed: {failure}"
            )));
        }
        let result = self.write_record_inner(value);
        if let Err(error) = &result {
            self.failed = Some(error.to_string());
        }
        result
    }

    fn write_record_inner(&mut self, mut value: serde_json::Value) -> std::io::Result<()> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("session diagnostic sequence overflow"))?;
        let Some(object) = value.as_object_mut() else {
            return Err(std::io::Error::other(
                "session diagnostic record is not an object",
            ));
        };
        let host_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                std::io::Error::other(format!("system clock precedes Unix epoch: {error}"))
            })?
            .as_micros();
        let host_us = u64::try_from(host_us)
            .map_err(|_| std::io::Error::other("host clock microseconds exceed u64"))?;
        let monotonic_us = u64::try_from(self.started.elapsed().as_micros())
            .map_err(|_| std::io::Error::other("monotonic microseconds exceed u64"))?;
        let clock_delta_us = self
            .last_wall_us
            .zip(self.last_mono_us)
            .map(|(wall, monotonic)| {
                (host_us as i128 - wall as i128) - (monotonic_us as i128 - monotonic as i128)
            });
        let clock_jump_detected =
            clock_delta_us.is_some_and(|delta| delta.unsigned_abs() > 250_000);
        object.insert("run_nonce".into(), self.run_nonce.clone().into());
        object.insert("server_process_id".into(), std::process::id().into());
        object.insert("sequence".into(), self.sequence.into());
        object.insert("host_unix_us".into(), host_us.into());
        object.insert("server_monotonic_us".into(), monotonic_us.into());
        object.insert(
            "wall_vs_monotonic_delta_us".into(),
            clock_delta_us.map(|value| value.to_string()).into(),
        );
        object.insert(
            "clock_correlation_error_bound_us".into(),
            250_000_u64.into(),
        );
        object.insert("clock_jump_detected".into(), clock_jump_detected.into());
        self.last_wall_us = Some(host_us.into());
        self.last_mono_us = Some(monotonic_us.into());
        let mut bytes = serde_json::to_vec(&value).map_err(std::io::Error::other)?;
        bytes.push(b'\n');
        self.file.write_all(&bytes)?;
        self.file.flush()?;
        Ok(())
    }

    pub fn hello_decision(
        &mut self,
        entity: Entity,
        player_id: u64,
        tick: u32,
        match_id: u64,
        has_live_session: bool,
        accepted_in_batch_before: bool,
        accepted_in_batch_after: bool,
        result: Result<SlotId, &str>,
        owners_before: &[Option<u64>],
        owners_after: &[Option<u64>],
    ) -> std::io::Result<()> {
        let decision = match result {
            Ok(slot) => {
                self.epoch(entity, player_id, Some(slot));
                serde_json::json!({"accepted": true, "slot": slot.0})
            }
            Err(reason) => {
                self.epoch(entity, player_id, None);
                serde_json::json!({"accepted": false, "reason": reason})
            }
        };
        let before = self.owner_snapshot(owners_before);
        let after = self.owner_snapshot(owners_after);
        let epoch = self.epochs.get(&entity.to_bits()).copied();
        self.write_record(serde_json::json!({
            "event": "hello_decision",
            "entity_bits_with_generation_decimal": exact_u64_decimal(entity.to_bits()),
            "connection_epoch_decimal": epoch.map(|value| exact_u64_decimal(value.epoch)),
            "identity_alias": epoch.map(|value| value.alias),
            "slot": epoch.and_then(|value| value.slot),
            "match_id_decimal": exact_u64_decimal(match_id),
            "sim_tick": tick,
            "has_live_session": has_live_session,
            "accepted_in_batch_before": accepted_in_batch_before,
            "accepted_in_batch_after": accepted_in_batch_after,
            "decision": decision,
            "owners_before": before,
            "owners_after": after,
        }))
    }

    pub fn session_lifecycle(
        &mut self,
        event: &'static str,
        entity: Entity,
        session: &Session,
        owners: &[Option<u64>],
        match_id: u64,
        tick: u32,
    ) -> std::io::Result<()> {
        let epoch = self.epoch(entity, session.player_id, Some(session.slot));
        let owner_snapshot = self.owner_snapshot(owners);
        self.write_record(serde_json::json!({
            "event": event,
            "entity_bits_with_generation_decimal": exact_u64_decimal(entity.to_bits()),
            "connection_epoch_decimal": exact_u64_decimal(epoch.epoch),
            "identity_alias": epoch.alias,
            "slot": session.slot.0,
            "match_id_decimal": exact_u64_decimal(match_id),
            "sim_tick": tick,
            "owners": owner_snapshot,
            "authoritative_session_present": event == "session_added",
        }))
    }

    pub fn owner_checkpoint(
        &mut self,
        owners: &[Option<u64>],
        sessions: impl Iterator<Item = (Entity, u64, SlotId)>,
        match_id: u64,
        tick: u32,
    ) -> std::io::Result<()> {
        let active_epochs: Vec<_> = sessions
            .map(|(entity, player_id, slot)| {
                let epoch = self.epoch(entity, player_id, Some(slot));
                serde_json::json!({
                    "entity_bits_with_generation_decimal": exact_u64_decimal(entity.to_bits()),
                    "connection_epoch_decimal": exact_u64_decimal(epoch.epoch),
                    "identity_alias": epoch.alias,
                    "slot": slot.0,
                    "match_id_decimal": exact_u64_decimal(match_id),
                })
            })
            .collect();
        let snapshot = self.owner_snapshot(owners);
        self.write_record(serde_json::json!({ "event":"owner_checkpoint", "sim_tick":tick,
            "match_id_decimal": exact_u64_decimal(match_id), "active_epochs":active_epochs, "owners":snapshot }))
    }
}

#[cfg(test)]
mod tests {
    use super::exact_u64_decimal;

    #[test]
    fn private_observer_u64_fields_have_exact_decimal_text() {
        assert_eq!(exact_u64_decimal(9_007_199_254_740_993), "9007199254740993");
        assert_eq!(exact_u64_decimal(u64::MAX), "18446744073709551615");
    }
}

pub fn on_session_added(
    event: On<Add<Session>>,
    sessions: Query<&Session>,
    slots: Res<Slots>,
    match_id: Res<MatchId>,
    tick: Res<Tick>,
    mut diagnostics: ResMut<SessionDiagnostics>,
) {
    let session = sessions
        .get(event.event().entity)
        .expect("added Session must be queryable by its observer");
    if let Err(error) = diagnostics.session_lifecycle(
        "session_added",
        event.event().entity,
        session,
        &slots.owners(),
        match_id.0,
        tick.0,
    ) {
        panic!("private session observer failed: {error}");
    }
}

pub fn on_session_removed(
    event: On<Remove<Session>>,
    sessions: Query<&Session>,
    slots: Res<Slots>,
    match_id: Res<MatchId>,
    tick: Res<Tick>,
    mut diagnostics: ResMut<SessionDiagnostics>,
) {
    let session = sessions
        .get(event.event().entity)
        .expect("removed Session must be queryable before its observer completes");
    if let Err(error) = diagnostics.session_lifecycle(
        "session_authority_removed",
        event.event().entity,
        session,
        &slots.owners(),
        match_id.0,
        tick.0,
    ) {
        panic!("private session observer failed: {error}");
    }
}
