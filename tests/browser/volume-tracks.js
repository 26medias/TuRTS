// turts-test-role: pure
"use strict";

// The volume sliders of the settings menu, found by pixels (RUN130). Product facts:
// crates/turts_client/src/volume_settings.rs: track colour srgb(0.20, 0.24, 0.30), 12 px high,
// thumb srgb(0.40, 0.75, 0.95), 20 px; the four volume controls are the last controls of the
// panel (client_shell.rs spawn_volume_controls after the bindings and settings).
// bevy_ui_widgets-0.20.0/src/slider.rs:347-350: a track click at x gives the value
// (x - left - thumb/2) / (width - thumb).
const TRACK_RGB = [51, 61, 77];
const THUMB_RGB = [102, 191, 242];
const COLOUR_TOLERANCE = 12;
const THUMB_SIZE = 20;
const VOLUME_CATEGORIES = ["master", "voice", "alerts", "effects"];

function near(image, x, y, rgb) {
    const offset = (y * image.width + x) * 4;
    return image.rgba[offset + 3] >= 245 &&
        Math.hypot(image.rgba[offset] - rgb[0], image.rgba[offset + 1] - rgb[1], image.rgba[offset + 2] - rgb[2]) <= COLOUR_TOLERANCE;
}

// Every horizontal slider track in the image, top to bottom: { y, left, width }. A track row has
// at least minWidth - THUMB_SIZE track pixels; a band of such rows is 10 to 14 rows high.
function findSliderTracks(image, minWidth = 150) {
    const rows = [];
    for (let y = 0; y < image.height; y += 1) {
        let count = 0;
        let left = Infinity;
        let right = -Infinity;
        for (let x = 0; x < image.width; x += 1) {
            const track = near(image, x, y, TRACK_RGB);
            if (track) count += 1;
            if (track || near(image, x, y, THUMB_RGB)) {
                left = Math.min(left, x);
                right = Math.max(right, x);
            }
        }
        rows.push(count >= minWidth - THUMB_SIZE ? { left, right } : null);
    }
    const tracks = [];
    for (let y = 0; y < rows.length;) {
        if (!rows[y]) { y += 1; continue; }
        const top = y;
        let left = Infinity;
        let right = -Infinity;
        while (y < rows.length && rows[y]) {
            left = Math.min(left, rows[y].left);
            right = Math.max(right, rows[y].right);
            y += 1;
        }
        const height = y - top;
        if (height >= 10 && height <= 14) tracks.push({ y: Math.round((top + y - 1) / 2), left, width: right - left + 1 });
    }
    return tracks;
}

// The four volume tracks: the last four slider tracks of the panel, or null.
function findVolumeTracks(image) {
    const tracks = findSliderTracks(image);
    if (tracks.length < VOLUME_CATEGORIES.length) return null;
    return Object.fromEntries(tracks.slice(-VOLUME_CATEGORIES.length).map((track, index) => [VOLUME_CATEGORIES[index], track]));
}

function volumeClickPoint(track, value) {
    return { x: Math.round(track.left + THUMB_SIZE / 2 + value * (track.width - THUMB_SIZE)), y: track.y };
}

module.exports = { findSliderTracks, findVolumeTracks, volumeClickPoint, VOLUME_CATEGORIES };
