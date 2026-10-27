// T7 browser join (design section 7; plan Task 12).
//
// Usage:
//   node t7.js <page url> <out dir> --ws <ws url> --expect-content-id <id> --expect-final <x,y>
//
// Opens <page url> in Chrome (CHROME_BIN, default /usr/bin/google-chrome) at 1280x720.
// M2_HEADLESS, M2_CHROME_ARGS_JSON, and M2_IGNORE_DEFAULT_ARGS_JSON configure the launch;
// without them it keeps the original headless/default-flag behavior. The page query is
// `?server=<origin of the page url>&ws=<ws url>`. The page url is the server's own HTTP
// address: the server serves the trunk build from its `static_dir`.
//
// The script then:
//   1. polls window.turts_debug() every 50 ms, for up to 30 s, until the phase starts with
//      "Joined". An empty string means that no frame has run yet; it is skipped.
//   2. calls window.turts_move(20, 8) 1 s after the join, and keeps polling for 6 s more.
//   3. writes <out dir>/frames.jsonl: one line per distinct debug string, with the page time of
//      the poll in milliseconds since the epoch (performance.timeOrigin + performance.now()),
//      the "last_frame" object, the "units" array and the "scene" object. The poll time is later
//      than the frame time, so the lag that the oracle works out from it is an upper bound.
//      The same poll reads window.turts_scene() (M1 verify):
//      {"cameras_3d": <number of active 3D cameras>, "units": [<the entities of the rendered
//      scene that have a mesh and a unit id, with their world translation>]}. Every recorded
//      poll must hold exactly one active 3D camera, because a scene that no camera shows is not
//      presented. It must also hold exactly one unit entity, with the unit id of the presented
//      unit, at the presented map position (x, y) mapped to the scene (x -> x, y -> z), within
//      0.01.
//   4. checks the last poll: one unit, the expected content id, and a position within 1e-3 of
//      --expect-final.
//   5. writes <out dir>/result.json. "scene_checked" is the number of polls whose scene passed
//      the check of step 3.
//
// Exit 0 when every check passes. Exit 1 on a failed check (the console log of the page is
// printed first). Exit 2 on a usage error.
const fs = require("fs");
const path = require("path");
const { chromium } = require("playwright-core");
const browserLaunchOptions = require("./t7-launch-options");

const JOIN_TIMEOUT_MS = 30000;
const POLL_MS = 50;
const MOVE_AFTER_JOIN_MS = 1000;
const RECORD_AFTER_MOVE_MS = 6000;
const MOVE_TARGET = [20, 8];
const POS_TOLERANCE = 1e-3;
const SCENE_TOLERANCE = 0.01;

function usage(message) {
    console.error(message);
    console.error(
        "usage: node t7.js <page url> <out dir> --ws <ws url> --expect-content-id <id> --expect-final <x,y>"
    );
    process.exit(2);
}

function parseArgs(argv) {
    const positional = [];
    const flags = {};
    for (let i = 0; i < argv.length; i++) {
        if (argv[i].startsWith("--")) {
            const value = argv[i + 1];
            if (value === undefined) {
                usage(`${argv[i]} needs a value`);
            }
            flags[argv[i]] = value;
            i++;
        } else {
            positional.push(argv[i]);
        }
    }
    const [pageUrl, outDir] = positional;
    for (const name of ["--ws", "--expect-content-id", "--expect-final"]) {
        if (!flags[name]) {
            usage(`${name} is required`);
        }
    }
    if (!pageUrl || !outDir || positional.length !== 2) {
        usage("expected a page url and an out dir");
    }
    const expectFinal = flags["--expect-final"].split(",").map(Number);
    if (expectFinal.length !== 2 || expectFinal.some((n) => !Number.isFinite(n))) {
        usage("--expect-final must be x,y");
    }
    return { pageUrl, outDir, ws: flags["--ws"], expectContentId: flags["--expect-content-id"], expectFinal };
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

(async () => {
    const args = parseArgs(process.argv.slice(2));
    fs.mkdirSync(args.outDir, { recursive: true });
    const url = new URL(args.pageUrl);
    url.searchParams.set("server", url.origin);
    url.searchParams.set("ws", args.ws);

    const executablePath = process.env.CHROME_BIN || "/usr/bin/google-chrome";
    const browser = await chromium.launch({ executablePath, ...browserLaunchOptions(process.env) });
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    const logs = [];
    page.on("console", (m) => logs.push(m.type() + ": " + m.text()));
    page.on("pageerror", (e) => logs.push("pageerror: " + e.message));
    page.on("requestfailed", (r) => logs.push("requestfailed: " + r.url() + " " + (r.failure() || {}).errorText));

    const result = {
        welcomed: false,
        units: 0,
        content_id: null,
        shape: null,
        final_pos: null,
        frames: 0,
        phase: null,
        scene_checked: 0,
        chrome: browser.version(),
        errors: [],
    };
    const finish = async (code) => {
        fs.writeFileSync(path.join(args.outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
        await browser.close();
        if (code !== 0) {
            for (const line of logs) {
                console.log(line);
            }
            for (const error of result.errors) {
                console.error("T7 FAIL: " + error);
            }
        }
        process.exit(code);
    };

    // One poll: the debug JSON and the page clock, read in the same call. `null` before the
    // first frame (the hook is absent or returns an empty string).
    const poll = async () => {
        const raw = await page.evaluate(() => ({
            at_ms: performance.timeOrigin + performance.now(),
            text: typeof window.turts_debug === "function" ? window.turts_debug() : null,
            scene: typeof window.turts_scene === "function" ? window.turts_scene() : null,
            error: window.turts_error || null,
        }));
        if (raw.error) {
            result.errors.push("window.turts_error: " + raw.error);
            return { failed: true };
        }
        if (!raw.text) {
            return null;
        }
        return { at_ms: raw.at_ms, text: raw.text, debug: JSON.parse(raw.text), scene: raw.scene ? JSON.parse(raw.scene) : null };
    };

    // The rendered scene of one poll against the presented units of the same poll. Returns
    // null when it passes, or the reason.
    const sceneProblem = (debug, scene) => {
        if (scene === null || typeof scene !== "object" || !Array.isArray(scene.units)) {
            return `window.turts_scene() gave ${JSON.stringify(scene)}`;
        }
        if (scene.cameras_3d !== 1) {
            return `expected exactly 1 active 3D camera, saw ${JSON.stringify(scene.cameras_3d)}`;
        }
        if (scene.units.length !== 1) {
            return `expected 1 scene entity with a mesh and a unit id, saw ${scene.units.length}`;
        }
        const unit = debug.units[0];
        const entity = scene.units[0];
        if (entity.unit_id !== unit.unit_id) {
            return `scene unit_id ${entity.unit_id} != presented unit_id ${unit.unit_id}`;
        }
        const [x, , z] = entity.translation;
        const off = [Math.abs(x - unit.pos[0]), Math.abs(z - unit.pos[1])];
        if (off.some((d) => !(d <= SCENE_TOLERANCE))) {
            return `scene translation ${JSON.stringify(entity.translation)} is not within ${SCENE_TOLERANCE} of presented pos ${JSON.stringify(unit.pos)} (x -> x, y -> z)`;
        }
        return null;
    };
    let sceneFailure = null;

    await page.goto(url.toString());

    // 1. Wait for the join.
    const startedAt = Date.now();
    let joinedAt = null;
    let latest = null;
    while (Date.now() - startedAt < JOIN_TIMEOUT_MS) {
        const seen = await poll();
        if (seen && seen.failed) {
            await finish(1);
        }
        if (seen) {
            latest = seen.debug;
            result.phase = latest.phase;
            if (latest.phase.startsWith("Joined")) {
                joinedAt = Date.now();
                break;
            }
            if (latest.phase.startsWith("Failed")) {
                break;
            }
        }
        await sleep(POLL_MS);
    }
    if (joinedAt === null) {
        result.errors.push(
            `never joined: phase ${JSON.stringify(result.phase)} after ${Date.now() - startedAt} ms`
        );
        await finish(1);
    }
    result.welcomed = true;

    // 2 and 3. Move, and record the polls.
    const frames = [];
    let previousText = null;
    let last = null;
    let movedAt = null;
    while (movedAt === null || Date.now() - movedAt < RECORD_AFTER_MOVE_MS) {
        if (movedAt === null && Date.now() - joinedAt >= MOVE_AFTER_JOIN_MS) {
            await page.evaluate(([x, y]) => window.turts_move(x, y), MOVE_TARGET);
            movedAt = Date.now();
        }
        const seen = await poll();
        if (seen && seen.failed) {
            await finish(1);
        }
        if (seen) {
            last = seen.debug;
            // A frame that the page has already shown to an earlier poll is not recorded twice.
            if (seen.text !== previousText && last.last_frame && last.units.length > 0) {
                frames.push(
                    JSON.stringify({ at_ms: seen.at_ms, last_frame: last.last_frame, units: last.units, scene: seen.scene })
                );
                previousText = seen.text;
                const problem = sceneProblem(last, seen.scene);
                if (problem === null) {
                    result.scene_checked++;
                } else if (sceneFailure === null) {
                    sceneFailure = `poll ${frames.length}: ${problem}`;
                }
            }
        }
        await sleep(POLL_MS);
    }
    fs.writeFileSync(path.join(args.outDir, "frames.jsonl"), frames.join("\n") + "\n");
    result.frames = frames.length;
    result.phase = last ? last.phase : result.phase;

    // 4. Checks on the last poll.
    const units = last ? last.units : [];
    result.units = units.length;
    if (units.length === 1) {
        result.content_id = units[0].content_id;
        result.shape = units[0].shape;
        result.final_pos = units[0].pos;
    }
    if (units.length !== 1) {
        result.errors.push(`expected 1 presented unit, saw ${units.length}`);
    } else {
        if (result.content_id !== args.expectContentId) {
            result.errors.push(`content_id ${result.content_id} != expected ${args.expectContentId}`);
        }
        const off = result.final_pos.map((v, i) => Math.abs(v - args.expectFinal[i]));
        if (off.some((d) => !(d <= POS_TOLERANCE))) {
            result.errors.push(
                `final position ${JSON.stringify(result.final_pos)} is not within ${POS_TOLERANCE} of ${JSON.stringify(args.expectFinal)}`
            );
        }
    }
    if (frames.length === 0) {
        result.errors.push("no frame was recorded");
    }
    if (sceneFailure !== null) {
        result.errors.push(
            `scene: ${sceneFailure} (${frames.length - result.scene_checked} of ${frames.length} polls failed)`
        );
    }
    console.log(JSON.stringify({ ...result, errors: undefined }));
    await finish(result.errors.length === 0 ? 0 : 1);
})().catch((e) => {
    console.error(e);
    process.exit(1);
});
