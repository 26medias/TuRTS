// turts-test-role: harness
"use strict";

// A Chrome that a test starts OUTSIDE the headed wrapper has no sound isolation: a page that plays sound plays it on
// the desktop of the operator. The wrapper (mc_headed_v6.sh) silences its Chrome with a null ALSA device and a pulse
// server that does not exist. This helper gives a test the same, with no `--mute-audio` and no change to the system
// sound: put `silentEnv(folder)` in the environment of the Chrome process and `SILENT_ARGS` in its arguments.
const fs = require("node:fs");
const path = require("node:path");

const SILENT_ARGS = ["--alsa-output-device=null"];
const NO_PULSE = "unix:/nonexistent/mc-tests-no-pulse";

// The environment for a Chrome: `base` with the sound isolation. A null ALSA configuration is written to `folder`.
function silentEnv(folder, base = process.env) {
    const config = path.join(folder, "alsa-null.conf");
    fs.writeFileSync(config, "</usr/share/alsa/alsa.conf>\npcm.!default { type null }\n");
    return { ...base, PULSE_SERVER: NO_PULSE, ALSA_CONFIG_PATH: config };
}

module.exports = { silentEnv, SILENT_ARGS, NO_PULSE };
