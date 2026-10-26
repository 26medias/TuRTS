//! Frame log.
//!
//! `--frame-log P` writes one line per presented unit per frame:
//! `frame_ns,unit_id,t,lo_tick,hi_tick,fraction,x,y,latest_tick`. At exit a summary line
//! follows: `# welcome_tick=<w> first_present_tick=<lo of the first record>
//! resync_requests_sent=<n> deltas_dropped=<t1;t2>`. A missing value is written as `none`.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use bevy::prelude::*;

use crate::present::{Presented, PresentedUnit};

/// The open frame log. It is closed when the path is `None`.
#[derive(Resource, Default)]
pub struct FrameLog {
    writer: Option<BufWriter<File>>,
    /// `lo_tick` of the first record that was written.
    pub first_present_tick: Option<u32>,
}

impl FrameLog {
    /// Opens (and empties) the file. A file that cannot be created is reported on stderr and
    /// the log stays closed, so the run goes on and the test that reads the log fails.
    pub fn open(path: Option<&Path>) -> Self {
        let writer = path.and_then(|path| match File::create(path) {
            Ok(file) => Some(BufWriter::new(file)),
            Err(error) => {
                eprintln!("turts_client: cannot create {}: {error}", path.display());
                None
            }
        });
        Self {
            writer,
            first_present_tick: None,
        }
    }

    /// Writes the summary line and flushes. Call it once, at exit.
    pub fn finish(&mut self, summary: &str) {
        if let Some(writer) = self.writer.as_mut() {
            let result = writeln!(writer, "{summary}").and_then(|()| writer.flush());
            if let Err(error) = result {
                eprintln!("turts_client: cannot write the frame log: {error}");
            }
        }
    }
}

/// One record of the frame log.
pub fn frame_line(presented: &Presented, unit: &PresentedUnit) -> String {
    format!(
        "{},{},{},{},{},{},{},{},{}",
        presented.frame_ns,
        unit.unit_id,
        presented.t,
        unit.lo_tick,
        unit.hi_tick,
        unit.fraction,
        unit.pos.x,
        unit.pos.y,
        presented.latest_tick,
    )
}

/// The summary line at the end of the frame log.
pub fn summary_line(
    welcome_tick: Option<u32>,
    first_present_tick: Option<u32>,
    resync_requests_sent: u32,
    deltas_dropped: &[u32],
) -> String {
    let tick = |tick: Option<u32>| tick.map_or("none".to_string(), |tick| tick.to_string());
    let dropped: Vec<String> = deltas_dropped.iter().map(u32::to_string).collect();
    format!(
        "# welcome_tick={} first_present_tick={} resync_requests_sent={resync_requests_sent} deltas_dropped={}",
        tick(welcome_tick),
        tick(first_present_tick),
        dropped.join(";"),
    )
}

/// `Update`, after `update_presented`: write the records of this frame.
pub fn write_frame_log(presented: Res<Presented>, mut log: ResMut<FrameLog>) {
    let Some(first) = presented.units.first() else {
        return;
    };
    if log.first_present_tick.is_none() {
        log.first_present_tick = Some(first.lo_tick);
    }
    let Some(writer) = log.writer.as_mut() else {
        return;
    };
    for unit in &presented.units {
        let result = writeln!(writer, "{}", frame_line(&presented, unit));
        if let Err(error) = result {
            eprintln!("turts_client: cannot write the frame log: {error}");
            return;
        }
    }
    // Flush each frame, so a reader sees a growing file and a killed client loses nothing.
    if let Err(error) = writer.flush() {
        eprintln!("turts_client: cannot write the frame log: {error}");
    }
}
