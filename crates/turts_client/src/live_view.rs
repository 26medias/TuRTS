//! Read-only live view of native Bevy presentation state for the existing `--out` test hook.
//!
//! This module is native-only and is scheduled only for windowed runs with `--out`. It observes
//! the permitted replica, actual UI ECS/layout, actual render entities/cameras, and selected
//! Bevy adapter. It never writes selection, gameplay state, or transport messages.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::ui::Pressed;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};

use crate::ClientOpts;
use crate::commands::CommandSendStatus;
use crate::debug::report_json;
use crate::economy_ui::{Selection, UiAction, UiActionTarget, rendered_ui_snapshot};
use crate::ingest::{CommandResult, CommandResults, ReplicaState};
use crate::join::Phase;
use crate::present::Presented;
use crate::render::{
    PresentedNode, PresentedUnit as RenderedUnit, declared_node_pick_radius,
    declared_unit_pick_radius, projected_ground_center,
};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Resource, Default)]
pub(crate) struct LiveReportCursor {
    tick: Option<u32>,
    phase: Option<String>,
}

/// Publish a complete owner-permitted report when its coherent tick changes.
pub(crate) fn publish_live_report(world: &mut World) {
    let path = world.resource::<ClientOpts>().0.live_report.clone();
    let Some(path) = path else { return };
    let tick = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.replica.as_ref())
        .and_then(|replica| replica.last_applied_tick);
    let phase = world
        .get_resource::<Phase>()
        .map(|phase| format!("{phase:?}"));
    let terminal = phase
        .as_deref()
        .is_some_and(|phase| phase.starts_with("Failed"));
    {
        let mut cursor = world.resource_mut::<LiveReportCursor>();
        if (tick.is_none() && !terminal) || (tick == cursor.tick && phase == cursor.phase) {
            return;
        }
        cursor.tick = tick;
        cursor.phase = phase;
    }
    if let Err(error) = write_json_atomic(&path, &report_json(world)) {
        eprintln!(
            "turts_client: cannot write live report {}: {error}",
            path.display()
        );
    }
}

/// The live view is a sibling of `--out PATH`, with no additional CLI option.
pub fn view_path(headless: bool, out: Option<&Path>) -> Option<PathBuf> {
    if headless {
        return None;
    }
    let mut sibling = OsString::from(out?.as_os_str());
    sibling.push(".view.json");
    Some(PathBuf::from(sibling))
}

/// Replace a complete JSON snapshot atomically on the destination filesystem.
pub fn write_json_atomic(path: &Path, value: &Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    let mut temp_path = path.as_os_str().to_owned();
    temp_path.push(format!(
        ".tmp.{}.{}",
        std::process::id(),
        NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let temp_path = PathBuf::from(temp_path);

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

/// Publish after `PostUpdate` UI layout and transform propagation have completed.
pub fn publish_live_view(world: &mut World) {
    let opts = &world.resource::<ClientOpts>().0;
    let Some(path) = view_path(opts.headless, opts.out.as_deref()) else {
        return;
    };
    if let Err(error) = write_json_atomic(&path, &snapshot_json(world)) {
        eprintln!(
            "turts_client: cannot write live view {}: {error}",
            path.display()
        );
    }
}

fn ui_snapshot(world: &World) -> Vec<Value> {
    let mut controls = std::collections::HashMap::<UiAction, (u64, Option<[f32; 4]>, bool)>::new();
    for entity in world.iter_entities() {
        if !entity.contains::<Button>() {
            continue;
        }
        let Some(action) = entity.get::<UiActionTarget>() else {
            continue;
        };
        let rect = entity
            .get::<ComputedNode>()
            .zip(entity.get::<UiGlobalTransform>())
            .map(|(node, transform)| {
                let center = transform.to_scale_angle_translation().2;
                let size = node.size();
                [
                    center.x - size.x / 2.0,
                    center.y - size.y / 2.0,
                    size.x,
                    size.y,
                ]
            });
        controls.insert(
            action.0.clone(),
            (entity.id().to_bits(), rect, entity.contains::<Pressed>()),
        );
    }

    rendered_ui_snapshot(world)
        .into_iter()
        .map(|node| {
            let control = node.action.as_ref().and_then(|action| controls.get(action));
            json!({
                "text": node.text,
                "rect": node.rect,
                "visible": node.visible,
                "enabled": node.enabled,
                "action": node.action,
                "button_entity": control.map(|control| control.0),
                "button_rect": control.and_then(|control| control.1),
                "button_pressed": control.map(|control| control.2),
            })
        })
        .collect()
}

fn snapshot_json(world: &World) -> Value {
    let state = world.get_resource::<ReplicaState>();
    let welcome = state.and_then(|state| state.welcome);
    let replica = state.and_then(|state| state.replica.as_ref());
    let acquired = state.and_then(|state| state.content.as_ref());

    let units: Vec<Value> = replica
        .into_iter()
        .flat_map(|replica| replica.units.iter())
        .map(|(&id, unit)| {
            let samples: Vec<Value> = unit
                .samples
                .iter()
                .map(|(tick, pos)| json!({ "tick": tick, "pos": [pos.x, pos.y] }))
                .collect();
            json!({
                "id": id,
                "content_id": unit.content_id,
                "owner_slot": unit.owner_slot,
                "samples": samples,
            })
        })
        .collect();
    let mut units = units;
    units.sort_by_key(|unit| unit["id"].as_u64().unwrap_or_default());

    let mut node_records: Vec<_> = replica
        .into_iter()
        .flat_map(|replica| replica.nodes.values())
        .collect();
    node_records.sort_by_key(|node| node.id.0);
    let nodes: Vec<Value> = node_records.into_iter().map(|node| json!(node)).collect();

    let world_state = replica.map(|replica| {
        json!({
            "coherent": replica.last_applied_tick.is_some(),
            "snapshot_tick": replica.snapshot_tick,
            "applied_tick": replica.last_applied_tick,
            "tick_hz": replica.tick_hz,
            "resync_active": replica.in_resync(),
            "resync_buffered_batches": replica.buffered_batches(),
            "highest_observed_tick": replica.highest_observed_tick,
            "buffer_overflows": replica.buffer_overflows,
            "units": units,
            "nodes": nodes,
            "economy": replica.economy,
        })
    });

    let windows: Vec<Value> = world
        .iter_entities()
        .filter_map(|entity| {
            let window = entity.get::<Window>()?;
            Some(json!({
                "entity": entity.id().to_bits(),
                "primary": entity.contains::<bevy::window::PrimaryWindow>(),
                "logical_size": [window.width(), window.height()],
                "physical_size": [window.resolution.physical_width(), window.resolution.physical_height()],
                "scale_factor": window.resolution.scale_factor(),
            }))
        })
        .collect();

    let cameras: Vec<Value> = world
        .iter_entities()
        .filter_map(|entity| {
            if !entity.contains::<Camera3d>() {
                return None;
            }
            let camera = entity.get::<Camera>()?;
            let transform = entity.get::<GlobalTransform>()?;
            let viewport_rect = camera.logical_viewport_rect();
            let rect =
                viewport_rect.map(|rect| [rect.min.x, rect.min.y, rect.width(), rect.height()]);
            Some(json!({
                "entity": entity.id().to_bits(),
                "active": camera.is_active,
                "viewport_logical": rect,
                "world_position": transform.translation().to_array(),
                "world_rotation": transform.to_scale_rotation_translation().1.to_array(),
            }))
        })
        .collect();
    let active_camera = world.iter_entities().find_map(|entity| {
        if !entity.contains::<Camera3d>() {
            return None;
        }
        let camera = entity
            .get::<Camera>()?
            .is_active
            .then(|| entity.get::<Camera>())??;
        let transform = entity.get::<GlobalTransform>()?;
        Some((entity.id().to_bits(), camera, transform))
    });

    let presented = world.get_resource::<Presented>();
    let meshes: Vec<Value> = world
        .iter_entities()
        .filter_map(|entity| {
            let mesh = entity.get::<Mesh3d>()?;
            let transform = entity.get::<GlobalTransform>()?;
            let marker = entity.get::<RenderedUnit>().map(|unit| ("unit", unit.0));
            let marker =
                marker.or_else(|| entity.get::<PresentedNode>().map(|node| ("node", node.0)))?;
            let (kind, id) = marker;
            let (map_pos, pick_radius) = if kind == "unit" {
                let unit = presented
                    .and_then(|presented| presented.units.iter().find(|unit| unit.unit_id == id));
                let pos = unit.map(|unit| unit.pos);
                let radius = unit.and_then(|unit| {
                    acquired
                        .and_then(|content| declared_unit_pick_radius(content, &unit.content_id))
                });
                (pos, radius)
            } else {
                let node =
                    replica.and_then(|replica| replica.nodes.values().find(|node| node.id.0 == id));
                let pos = node.map(|node| node.pos);
                let radius = node.and_then(|node| {
                    acquired
                        .and_then(|content| declared_node_pick_radius(content, &node.content_id))
                });
                (pos, radius)
            };
            let ground_center = active_camera.and_then(|(_, camera, camera_transform)| {
                map_pos.and_then(|pos| projected_ground_center(camera, camera_transform, pos))
            });
            let mesh_center = active_camera.and_then(|(_, camera, camera_transform)| {
                camera
                    .world_to_viewport(camera_transform, transform.translation())
                    .ok()
            });
            Some(json!({
                "entity": entity.id().to_bits(),
                "camera_entity": active_camera.map(|(camera_entity, _, _)| camera_entity),
                "kind": kind,
                "id": id,
                "mesh_asset": format!("{:?}", mesh.0.id()),
                "world_transform": transform.to_matrix().to_cols_array(),
                "world_position": transform.translation().to_array(),
                "map_pos": map_pos.map(|pos| [pos.x, pos.y]),
                "projected_ground_center": ground_center.map(|point| [point.x, point.y]),
                "projected_mesh_center": mesh_center.map(|point| [point.x, point.y]),
                "pick_radius_world": pick_radius,
            }))
        })
        .collect();

    let command_results: Vec<Value> = world
        .get_resource::<CommandResults>()
        .into_iter()
        .flat_map(|results| results.0.iter())
        .map(|result| match result {
            CommandResult::Acked { seq, tick } => {
                json!({"seq": seq, "kind": "acked", "tick": tick})
            }
            CommandResult::Applied { seq, tick, job_id } => json!({
                "seq": seq,
                "kind": "applied",
                "tick": tick,
                "job_id": job_id.map(|job| job.0),
            }),
            CommandResult::Rejected { seq, tick, reason } => json!({
                "seq": seq,
                "kind": "rejected",
                "tick": tick,
                "reason": reason.as_str(),
            }),
        })
        .collect();

    let adapter = world.get_resource::<RenderAdapterInfo>().map(|adapter| {
        json!({
            "name": adapter.name,
            "backend": format!("{:?}", adapter.backend),
            "device_type": format!("{:?}", adapter.device_type),
            "vendor": adapter.vendor,
            "device": adapter.device,
            "driver": adapter.driver,
            "driver_info": adapter.driver_info,
        })
    });

    json!({
        "schema": 1,
        "phase": world.get_resource::<Phase>().map_or_else(|| "unknown".to_string(), |phase| format!("{phase:?}")),
        "welcome": welcome.map(|welcome| json!({
            "slot": welcome.slot,
            "tick": welcome.tick,
            "tick_hz": welcome.tick_hz,
        })),
        "world": world_state,
        "presentation": presented.map(|presented| json!({
            "tick": presented.latest_tick,
            "time": presented.t,
            "frame_ns": presented.frame_ns,
            "unit_count": presented.units.len(),
        })),
        "selection": world.get_resource::<Selection>().and_then(|selection| selection.0),
        "command_results": command_results,
        "reconnect_required": world.get_resource::<CommandSendStatus>().is_some_and(|status| status.reconnect_required),
        "ui": ui_snapshot(world),
        "windows": windows,
        "cameras": cameras,
        "meshes": meshes,
        "render_adapter": adapter,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::{view_path, write_json_atomic};

    #[test]
    fn live_view_path_requires_windowed_run_and_existing_out_option() {
        let out = Path::new("/tmp/native-report.json");
        assert_eq!(view_path(true, Some(out)), None);
        assert_eq!(view_path(false, None), None);
        assert_eq!(
            view_path(false, Some(out)),
            Some(PathBuf::from("/tmp/native-report.json.view.json"))
        );
    }

    #[test]
    fn snapshot_json_contains_actual_ui_text_and_the_coherent_replica() {
        use std::collections::VecDeque;

        use bevy::prelude::{Text, World};
        use bevy::ui::Pressed;
        use bevy::ui_widgets::Button;
        use turts_net_protocol::Pos;

        use crate::economy_ui::{Selection, UiAction, UiActionTarget, UiSnapshotText};
        use crate::ingest::{CommandResults, ReplicaState, WelcomeInfo};
        use crate::join::Phase;
        use crate::present::Presented;
        use crate::replica::{Replica, UnitReplica};

        let mut world = World::new();
        let mut replica = Replica::new(20);
        replica.last_applied_tick = Some(9);
        replica.units.insert(
            7,
            UnitReplica {
                content_id: "test:builder".into(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::from([(9, Pos { x: 12.0, y: 8.0 })]),
            },
        );
        world.insert_resource(ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 9,
                tick_hz: 20,
            }),
            ..Default::default()
        });
        world.insert_resource(Phase::Joined { slot: 0 });
        world.insert_resource(Presented::default());
        world.insert_resource(Selection(Some(7)));
        world.insert_resource(CommandResults::default());
        let action = UiAction::Produce {
            producer: 7,
            recipe: "test:worker".into(),
        };
        world.spawn((Button, UiActionTarget(action.clone()), Pressed));
        world.spawn((
            UiSnapshotText,
            Text::new("Actual rendered panel text"),
            UiActionTarget(action),
        ));

        let snapshot = super::snapshot_json(&world);
        assert_eq!(snapshot["world"]["coherent"], true);
        assert_eq!(snapshot["world"]["applied_tick"], 9);
        assert_eq!(snapshot["world"]["units"][0]["id"], 7);
        assert_eq!(
            snapshot["world"]["units"][0]["samples"][0]["pos"],
            json!([12.0, 8.0])
        );
        assert_eq!(snapshot["selection"], 7);
        let action_row = snapshot["ui"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["text"] == "Actual rendered panel text")
            .expect("actual text row is exported");
        assert_eq!(action_row["button_pressed"], true);
        assert_eq!(snapshot["render_adapter"], serde_json::Value::Null);
    }

    #[test]
    fn snapshot_writer_atomically_replaces_a_complete_json_sibling() {
        let dir = std::env::temp_dir().join(format!("turts-live-view-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create test directory");
        let out = dir.join("report.json");
        let path = view_path(false, Some(&out)).expect("windowed out path enables view");
        write_json_atomic(&path, &json!({"sequence": 1, "complete": true}))
            .expect("write first complete snapshot");
        write_json_atomic(&path, &json!({"sequence": 2, "complete": true}))
            .expect("replace with second complete snapshot");
        let parsed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read snapshot"))
                .expect("snapshot is valid JSON");
        assert_eq!(parsed, json!({"sequence": 2, "complete": true}));
        assert_eq!(
            std::fs::read_dir(&dir)
                .expect("list test directory")
                .count(),
            1
        );
        std::fs::remove_dir_all(dir).expect("remove test directory");
    }
}
