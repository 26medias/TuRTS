// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng } = require("./harness/png.js");

async function judge(view, params) {
    assert.equal(params.capture_scene_checkpoints, true, "GM40 pixels require C84 scene checkpoints after screenshots");
    assert.equal(params.capture_order_log, true, "GM40 pixels require the actual same-run server order-transition log");
    assert.equal(params.order_log_file, "server-order.log", "GM40 uses the fixed actual server order-log artifact name");
    const input = view.inputLog();
    const targetEvidence = JSON.parse(await view.serverFile("gm40-pixel-target.json"));
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    const tickRows = parseTickLog(await view.serverFile("server-tick.log"));
    const orderRows = parseOrderLog(await view.serverFile(params.order_log_file));
    const serverArgv = JSON.parse(await view.serverFile("server-argv.json"));
    const orderFlag = serverArgv.indexOf("--order-log");
    assert(orderFlag >= 0 && typeof serverArgv[orderFlag + 1] === "string" &&
        serverArgv[orderFlag + 1].endsWith(`/${params.order_log_file}`),
    "GM40 actual launcher provenance binds --order-log to the published same-run artifact");
    assert(Array.isArray(checkpoints) && checkpoints.length > 0, "GM40 needs actual C84 scene/debug checkpoints");
    const boxAction = input.find((entry) => entry.call === "drag" && entry.args?.[4] === "left");
    const rightClick = input.find((entry) => entry.call === "click" && entry.args?.[2] === "right");
    assert(boxAction, "GM40 must use a physical left-button selection box");
    assert(rightClick, "GM40 must send an actual right click at a ground point");
    assert.deepEqual(rightClick.args.slice(0, 2), [targetEvidence.target_pixel.x, targetEvidence.target_pixel.y],
        "GM40 right click uses the pixel-only target captured by the driver");
    for (const [name, action] of [["box", boxAction], ["right-click", rightClick]]) {
        const down = action.actual_events?.find((event) => event.type === "pointerdown");
        const up = action.actual_events?.find((event) => event.type === "pointerup");
        assert(down?.canvas_target && up?.canvas_target && down.seq < up.seq,
            `GM40 ${name} must be an ordered physical pointer gesture on the canvas`);
    }

    const afterBox = screenshotCheckpoint(input, checkpoints, "gm40-after-box.png");
    const afterArrival = screenshotCheckpoint(input, checkpoints, "gm40-after-arrival.png");
    const afterRest = screenshotCheckpoint(input, checkpoints, "gm40-after-rest.png");
    const selected = actualTrooperSelection(afterBox);
    const ids = new Set(selected.map((unit) => unit.unit_id));
    const actualTarget = unprojectGround(afterArrival.scene, targetEvidence.target_pixel);
    const targetPixelScene = afterArrival.scene;
    assert(actualTarget.every(Number.isFinite), "GM40 click unprojects to a finite ground point through the actual C84 camera");

    assertRestWindow(tickRows, ids, 40, actualTarget);
    assertSceneGeometry(afterRest.scene, ids, actualTarget, 0.5);
    assertSceneStable(afterArrival.scene, afterRest.scene, ids, 0.001);
    await assertActualPngs(view, selected.length, params);
    assertNoBlocked(orderRows, ids);
    assert.equal(targetPixelScene.viewport_logical_rect?.length, 4, "GM40 target unprojection uses actual viewport metadata");
    assert.equal(targetPixelScene.viewport_logical_rect[2], params.viewport.width, "GM40 scene viewport width matches the declared capture");
    assert.equal(targetPixelScene.viewport_logical_rect[3], params.viewport.height, "GM40 scene viewport height matches the declared capture");
}

function screenshotCheckpoint(input, checkpoints, filename) {
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === filename);
    assert(saveIndex > 0, `GM40 ${filename} must be saved from an actual screenshot`);
    let screenshotIndex = saveIndex - 1;
    while (screenshotIndex >= 0 && input[screenshotIndex].call !== "screenshot") screenshotIndex -= 1;
    assert(screenshotIndex >= 0, `GM40 ${filename} has no screenshot input`);
    const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
    assert(checkpoint?.debug && checkpoint?.scene, `GM40 ${filename} has no actual C84 scene/debug checkpoint`);
    return checkpoint;
}

function actualTrooperSelection(checkpoint) {
    const selection = checkpoint.debug.selection;
    assert(Array.isArray(selection), "GM40 C84 debug state has no actual selection feed");
    assert.equal(selection.length, 40, "GM40 physical box selects exactly forty actual units");
    const replica = checkpoint.debug.replicated_world?.units;
    assert(Array.isArray(replica), "GM40 C84 checkpoint has no actual replicated unit catalog");
    const byId = new Map(replica.map((unit) => [unit.unit_id, unit]));
    const selected = selection.map((id) => {
        const unit = byId.get(id);
        assert(unit, `GM40 selected id ${id} is absent from the actual replica`);
        assert.equal(unit.content_id, "muster:trooper", `GM40 selected id ${id} is not an actual Muster trooper`);
        assert.equal(unit.owner_slot, checkpoint.debug.slot, `GM40 selected trooper ${id} is not owned by the local slot`);
        return unit;
    });
    assert.equal(new Set(selection).size, 40, "GM40 selection has forty distinct actual IDs");
    const rendered = checkpoint.scene.units;
    assert(Array.isArray(rendered), "GM40 scene has no actual rendered unit records");
    for (const id of selection) {
        const entry = rendered.find((unit) => unit.unit_id === id);
        assert(entry, `GM40 selected troop ${id} has no actual scene entry`);
        assert(Array.isArray(entry.translation) && entry.translation.length === 3 && entry.translation.every(Number.isFinite),
            `GM40 selected troop ${id} has no finite actual translation`);
        assert(Array.isArray(entry.ground_screen_center) && entry.ground_screen_center.length === 2 && entry.ground_screen_center.every(Number.isFinite),
            `GM40 selected troop ${id} has no actual ground_screen_center`);
        assert(Number.isFinite(entry.pick_radius) && entry.pick_radius > 0,
            `GM40 selected troop ${id} has no actual pick_radius`);
    }
    return selected;
}

function assertSceneGeometry(scene, ids, target, radius) {
    const units = scene.units;
    assert(Array.isArray(units), "GM40 final scene has no actual units");
    const world = [...ids].map((id) => {
        const entry = units.find((unit) => unit.unit_id === id);
        assert(entry, `GM40 final scene omits selected unit ${id}`);
        const center = unprojectGround(scene, { x: entry.ground_screen_center[0], y: entry.ground_screen_center[1] });
        return { id, center, radius: entry.pick_radius };
    });
    for (const unit of world) {
        assert(Math.hypot(unit.center[0] - target[0], unit.center[2] - target[2]) <= 4.58,
            `GM40 unit ${unit.id} is farther than R_near=4.58 from the actual clicked point`);
    }
    for (let left = 0; left < world.length; left += 1) {
        for (let right = left + 1; right < world.length; right += 1) {
            const a = world[left];
            const b = world[right];
            const distance = Math.hypot(a.center[0] - b.center[0], a.center[2] - b.center[2]);
            assert(distance >= a.radius + b.radius - 0.001,
                `GM40 actual projected ground discs ${a.id}/${b.id} overlap`);
        }
    }
}

function assertSceneStable(before, after, ids, epsilon) {
    const find = (scene, id) => scene.units.find((unit) => unit.unit_id === id);
    for (const id of ids) {
        const a = find(before, id);
        const b = find(after, id);
        assert(a && b, `GM40 stability checkpoints omit selected unit ${id}`);
        const aw = unprojectGround(before, { x: a.ground_screen_center[0], y: a.ground_screen_center[1] });
        const bw = unprojectGround(after, { x: b.ground_screen_center[0], y: b.ground_screen_center[1] });
        assert(Math.hypot(aw[0] - bw[0], aw[2] - bw[2]) <= epsilon,
            `GM40 selected unit ${id} moved over the browser's >2 s rest window`);
    }
}

function assertRestWindow(rows, ids, count, target) {
    const byTick = new Map();
    for (const row of rows) {
        if (!ids.has(row.unit_id)) continue;
        if (!byTick.has(row.tick)) byTick.set(row.tick, new Map());
        byTick.get(row.tick).set(row.unit_id, [row.x, row.y]);
    }
    const ticks = [...byTick.keys()].sort((a, b) => a - b);
    assert(ticks.length >= count + 1, `GM40 tick log has fewer than ${count + 1} samples for ${count} actual tick intervals`);
    const end = ticks.at(-1);
    const first = end - count;
    const baseline = byTick.get(first);
    assert(baseline, "GM40 rest interval has no actual starting tick");
    for (let tick = first; tick <= end; tick += 1) {
        const current = byTick.get(tick);
        assert(current, `GM40 tick log omits rest tick ${tick}`);
        for (const id of ids) {
            const pos = current.get(id);
            assert(pos, `GM40 tick ${tick} omits selected unit ${id}`);
            assert(Math.hypot(pos[0] - target[0], pos[1] - target[2]) <= 4.58,
                `GM40 tick-log unit ${id} exceeds R_near from the actual target at tick ${tick}`);
            const origin = baseline.get(id);
            assert(origin, `GM40 rest baseline omits selected unit ${id}`);
            assert(Math.hypot(pos[0] - origin[0], pos[1] - origin[1]) <= 0.001,
                `GM40 tick-log unit ${id} changed position by >0.001 during its 2 s rest window`);
        }
        assertNoDiscOverlap(current, ids, 0.5, tick);
    }
}

function assertNoDiscOverlap(positions, ids, radius, tick) {
    const ordered = [...ids].sort((a, b) => a - b);
    for (let left = 0; left < ordered.length; left += 1) {
        for (let right = left + 1; right < ordered.length; right += 1) {
            const a = positions.get(ordered[left]);
            const b = positions.get(ordered[right]);
            assert(a && b, `GM40 tick ${tick} lacks a selected position`);
            assert(Math.hypot(a[0] - b[0], a[1] - b[1]) >= 2 * radius - 0.001,
                `GM40 tick-log units ${ordered[left]}/${ordered[right]} overlap at tick ${tick}`);
        }
    }
}

function parseOrderLog(text) {
    assert.equal(typeof text, "string", "GM40 order log is readable as text");
    assert(text.endsWith("\n"), "GM40 actual order log is incomplete");
    const rows = text.trimEnd().split("\n").filter(Boolean).map((line) => {
        const fields = line.split(",");
        assert.equal(fields.length, 4, `GM40 order-log schema must be tick,unit_id,state,reason: ${line}`);
        const row = { tick: Number(fields[0]), unit_id: Number(fields[1]), state: fields[2], reason: fields[3] };
        assert(Number.isInteger(row.tick) && Number.isInteger(row.unit_id) && row.state.length > 0 && row.reason.length > 0,
            `GM40 order log has malformed transition: ${line}`);
        return row;
    });
    for (let index = 1; index < rows.length; index += 1) {
        const previous = rows[index - 1];
        const current = rows[index];
        assert(previous.tick < current.tick || (previous.tick === current.tick && previous.unit_id < current.unit_id),
            "GM40 actual order transitions are strictly ordered by tick and then unit ID");
    }
    return rows;
}

function assertNoBlocked(rows, ids) {
    for (const id of ids) {
        const transitions = rows.filter((row) => row.unit_id === id);
        assert(transitions.some((row) => row.state === "Moving"), `GM40 pixel unit ${id} has no actual Moving transition`);
        assert(transitions.some((row) => row.state === "Done"), `GM40 pixel unit ${id} has no actual Done transition`);
        assert(!transitions.some((row) => row.state === "Blocked"), `GM40 pixel unit ${id} transitioned to Blocked`);
    }
}

function parseTickLog(text) {
    assert.equal(typeof text, "string");
    assert(text.endsWith("\n"), "GM40 actual tick log is incomplete");
    const rows = text.trimEnd().split("\n").map((line) => {
        const fields = line.split(",");
        assert.equal(fields.length, 5, `GM40 tick log schema must be tick,wall_ns,unit_id,x,y: ${line}`);
        return { tick: Number(fields[0]), wall_ns: fields[1], unit_id: Number(fields[2]), x: Number(fields[3]), y: Number(fields[4]) };
    });
    assert(rows.length > 0 && rows.every((row) => Number.isInteger(row.tick) && Number.isInteger(row.unit_id) && Number.isFinite(row.x) && Number.isFinite(row.y)),
        "GM40 actual tick log has invalid values");
    return rows;
}

async function assertActualPngs(view, expectedSelection, params) {
    const before = decodePng(Buffer.from(await view.serverFile("gm40-before.png.b64"), "base64"));
    const selected = decodePng(Buffer.from(await view.serverFile("gm40-after-box.png.b64"), "base64"));
    const rest = decodePng(Buffer.from(await view.serverFile("gm40-after-rest.png.b64"), "base64"));
    const width = params.viewport?.width;
    const height = params.viewport?.height;
    assert.equal(width, 1280, "GM40 row declares the fixed 1280-pixel viewport width");
    assert.equal(height, 720, "GM40 row declares the fixed 720-pixel viewport height");
    assert.equal(before.width, selected.width);
    assert.equal(before.height, selected.height);
    assert.equal(selected.width, rest.width);
    assert.equal(selected.height, rest.height);
    for (const [name, image] of [["before", before], ["after-box", selected], ["after-rest", rest]]) {
        assert.equal(image.width, width, `GM40 ${name} PNG width matches the declared viewport`);
        assert.equal(image.height, height, `GM40 ${name} PNG height matches the declared viewport`);
    }
    assert.equal(expectedSelection, 40);
    assert(before.rgba.length > 0 && selected.rgba.length > 0 && rest.rgba.length > 0, "GM40 retains the actual before, post-box, and at-rest PNG captures");
    assertRenderedWorldIsNonflat(rest);
}

function assertRenderedWorldIsNonflat(image) {
    const colors = new Set();
    const stepX = Math.max(1, Math.floor(image.width / 64));
    const stepY = Math.max(1, Math.floor(image.height / 36));
    for (let y = 0; y < image.height; y += stepY) {
        for (let x = 0; x < image.width; x += stepX) {
            const offset = (y * image.width + x) * 4;
            if (image.rgba[offset + 3] === 0) continue;
            colors.add(`${image.rgba[offset]},${image.rgba[offset + 1]},${image.rgba[offset + 2]}`);
            if (colors.size >= 8) return;
        }
    }
    assert(colors.size >= 8, "GM40 final PNG must contain a rendered, nonflat world capture");
}

// C84 actual camera unprojection; the driver never reads scene or debug metadata.
function unprojectGround(scene, pixel) {
    const matrix = scene?.clip_from_world;
    const viewport = scene?.viewport_logical_rect;
    assert(Array.isArray(matrix) && matrix.length === 16 && matrix.every(Number.isFinite), "GM40 actual camera clip_from_world is missing or invalid");
    assert(Array.isArray(viewport) && viewport.length === 4 && viewport.every(Number.isFinite) && viewport[2] > 0 && viewport[3] > 0,
        "GM40 actual viewport_logical_rect is missing or invalid");
    const x = ((pixel.x - viewport[0]) / viewport[2]) * 2 - 1;
    const y = 1 - ((pixel.y - viewport[1]) / viewport[3]) * 2;
    const inverse = invert(matrix);
    const near = transform(inverse, [x, y, -1, 1]);
    const far = transform(inverse, [x, y, 1, 1]);
    assert(Math.abs(near[3]) > 1e-12 && Math.abs(far[3]) > 1e-12, "GM40 C84 unprojection has a finite homogeneous coordinate");
    const a = near.slice(0, 3).map((value) => value / near[3]);
    const b = far.slice(0, 3).map((value) => value / far[3]);
    const dy = b[1] - a[1];
    assert(Math.abs(dy) > 1e-12, "GM40 screen ray is parallel to the ground plane");
    const t = -a[1] / dy;
    const point = a.map((value, index) => value + (b[index] - value) * t);
    return point;
}

function invert(columnMajor) {
    const rows = Array.from({ length: 4 }, (_, row) => [
        columnMajor[row], columnMajor[4 + row], columnMajor[8 + row], columnMajor[12 + row],
        row === 0 ? 1 : 0, row === 1 ? 1 : 0, row === 2 ? 1 : 0, row === 3 ? 1 : 0,
    ]);
    for (let column = 0; column < 4; column += 1) {
        let pivot = column;
        for (let row = column + 1; row < 4; row += 1) if (Math.abs(rows[row][column]) > Math.abs(rows[pivot][column])) pivot = row;
        assert(Math.abs(rows[pivot][column]) > 1e-15, "GM40 actual camera matrix is singular");
        [rows[column], rows[pivot]] = [rows[pivot], rows[column]];
        const divisor = rows[column][column];
        for (let c = 0; c < 8; c += 1) rows[column][c] /= divisor;
        for (let row = 0; row < 4; row += 1) if (row !== column) {
            const factor = rows[row][column];
            for (let c = 0; c < 8; c += 1) rows[row][c] -= factor * rows[column][c];
        }
    }
    const result = Array(16);
    for (let row = 0; row < 4; row += 1) for (let col = 0; col < 4; col += 1) result[col * 4 + row] = rows[row][4 + col];
    return result;
}

function transform(matrix, vector) {
    return Array.from({ length: 4 }, (_, row) =>
        matrix[row] * vector[0] + matrix[4 + row] * vector[1] + matrix[8 + row] * vector[2] + matrix[12 + row] * vector[3]);
}

module.exports = { judge, unprojectGround, parseTickLog, parseOrderLog, assertRestWindow, assertSceneGeometry };
