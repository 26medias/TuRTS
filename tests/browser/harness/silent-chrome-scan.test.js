// turts-test-role: unit
"use strict";

// A test that starts a Chrome outside the headed wrapper must give it the sound isolation of harness/silent-chrome.js
// (the operator heard beeps on the desktop from a Chrome that had none). The scan reads every `*.test.js` file under
// tests/browser and finds each place that starts a Chrome:
//   - `spawn(` or `execFile(` whose first argument names the Chrome binary (the word CHROME or chrome in it),
//   - `chromium.launch(`, `launchPersistentContext(` and `connectOverCDP` are not starts; a `.launch(` is,
//   - a shim script that execs the Chrome binary (`exec '${CHROME}'`).
// Each such call must show `silentEnv(` and `SILENT_ARGS` (or `--alsa-output-device=null`) in its own text. A shim must
// carry `--alsa-output-device=null` on its exec line. Known gap, as for the driver rule: a Chrome that a helper
// function starts out of sight of the scan (the call text is in another file) passes; review must catch it.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { silentEnv, SILENT_ARGS, NO_PULSE } = require("./silent-chrome.js");

const BROWSER = path.resolve(__dirname, "..");

// The text of the call that starts at `open` (the index of "("), up to the matching ")". Strings are skipped.
function callText(source, open) {
    let depth = 0;
    for (let i = open; i < source.length; i += 1) {
        const char = source[i];
        if (char === "'" || char === '"' || char === "`") {
            for (i += 1; i < source.length && source[i] !== char; i += 1) if (source[i] === "\\") i += 1;
        } else if (char === "(") depth += 1;
        else if (char === ")") { depth -= 1; if (depth === 0) return source.slice(open, i + 1); }
    }
    return source.slice(open);
}

// The Chrome starts of a source that lack the isolation: a list of short descriptions.
function unsilenced(source) {
    const found = [];
    for (const match of source.matchAll(/\b(spawn|execFile|spawnSync)\(/g)) {
        const open = match.index + match[0].length - 1;
        const text = callText(source, open);
        const first = text.slice(1).split(",")[0];
        if (!/chrome/i.test(first)) continue;
        if (!/silentEnv\(/.test(text) || !(/SILENT_ARGS/.test(text) || /--alsa-output-device=null/.test(text))) found.push(`${match[1]}(${first.trim()}) has no silentEnv and SILENT_ARGS`);
    }
    for (const match of source.matchAll(/\.launch\(/g)) {
        const text = callText(source, match.index + match[0].length - 1);
        if (!/silentEnv\(/.test(text) || !/SILENT_ARGS/.test(text)) found.push(".launch( has no silentEnv and SILENT_ARGS");
    }
    for (const line of source.split("\n")) {
        if (/exec\s+'?\$\{CHROME\}/.test(line) && !/--alsa-output-device=null/.test(line)) found.push(`a shim execs the Chrome with no --alsa-output-device=null: ${line.trim().slice(0, 60)}`);
    }
    return found;
}

function testFiles(directory) {
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        if (entry.name === "node_modules") return [];
        const full = path.join(directory, entry.name);
        if (entry.isDirectory()) return testFiles(full);
        return entry.name.endsWith(".test.js") ? [full] : [];
    });
}

test("every test that starts a Chrome gives it the sound isolation", () => {
    const bad = testFiles(BROWSER).filter((file) => !file.endsWith("silent-chrome-scan.test.js"))
        .flatMap((file) => unsilenced(fs.readFileSync(file, "utf8")).map((what) => `${path.relative(BROWSER, file)}: ${what}`));
    assert.deepEqual(bad, []);
});

test("the scan finds a Chrome start with no isolation, in each form, and passes one that has it", () => {
    assert.equal(unsilenced("spawn(CHROME, [`--user-data-dir=${p}`, '--headless=new'], { stdio: 'ignore' });").length, 1);
    assert.equal(unsilenced("spawn(CHROME, [...SILENT_ARGS, 'x'], { env: process.env });").length, 1, "args alone are not enough");
    assert.equal(unsilenced("spawn(CHROME, ['x'], { env: silentEnv(dir) });").length, 1, "env alone is not enough");
    assert.equal(unsilenced("spawn(CHROME, [...SILENT_ARGS, 'x'], { env: silentEnv(dir) });").length, 0);
    assert.equal(unsilenced("execFile('/usr/bin/google-chrome', ['--headless'], cb)").length, 1);
    assert.equal(unsilenced("spawn('bash', [PLAY], { env: { CHROME_BIN: shim } })").length, 0, "a call that does not name the Chrome binary is not a start");
    assert.equal(unsilenced("await chromium.launch({ executablePath: CHROME, headless: true });").length, 1);
    assert.equal(unsilenced("await chromium.launch({ executablePath: CHROME, args: SILENT_ARGS, env: silentEnv(dir) });").length, 0);
    assert.equal(unsilenced("fs.writeFileSync(shim, `#!/bin/sh\\nexec '${CHROME}' --headless=new \"$@\"\\n`);").length, 1);
    assert.equal(unsilenced("fs.writeFileSync(shim, `#!/bin/sh\\nexec '${CHROME}' --headless=new --alsa-output-device=null \"$@\"\\n`);").length, 0);
    assert.equal(unsilenced("const x = connectOverCDP(url);").length, 0);
});

test("silentEnv sets a pulse server that does not exist and a null ALSA device, and mutes nothing", () => {
    const folder = fs.mkdtempSync(path.join(os.tmpdir(), "silent-chrome-"));
    try {
        const env = silentEnv(folder, { KEEP: "1" });
        assert.equal(env.KEEP, "1");
        assert.equal(env.PULSE_SERVER, NO_PULSE);
        assert.equal(env.PULSE_SERVER, "unix:/nonexistent/mc-tests-no-pulse", "the value of the wrapper");
        assert.equal(env.ALSA_CONFIG_PATH, path.join(folder, "alsa-null.conf"));
        assert.equal(fs.readFileSync(env.ALSA_CONFIG_PATH, "utf8"), "</usr/share/alsa/alsa.conf>\npcm.!default { type null }\n");
        assert.deepEqual(SILENT_ARGS, ["--alsa-output-device=null"]);
        assert.ok(!SILENT_ARGS.some((arg) => /mute/.test(arg)), "no --mute-audio");
    } finally { fs.rmSync(folder, { recursive: true, force: true }); }
});

// ---- Shell scripts under scripts/ ---------------------------------------------------------------------

// A script that starts a Chrome itself (`"$CHROME_BIN" "${ARGS[@]}"`, `google-chrome ... &`, `exec chromium`) must have the silent
// form: the argument --alsa-output-device=null and the silent environment (a pulse server that does not exist and a null ALSA
// configuration). The operator heard beeps from a Chrome that a script started with neither.
function scriptProblems(source) {
    const code = source.split("\n").filter((line) => !/^\s*#/.test(line)).join("\n");
    const starts = /"\$CHROME_BIN"\s+"\$\{|\b(?:exec\s+)?(?:google-chrome|chromium)(?:-browser)?\s+--[a-z]/.test(code);
    if (!starts) return [];
    const problems = [];
    if (!/--alsa-output-device=null/.test(code)) problems.push("starts a Chrome and never gives --alsa-output-device=null");
    if (!/PULSE_SERVER=.?unix:\/nonexistent\//.test(code)) problems.push("starts a Chrome and never sets PULSE_SERVER to a server that does not exist");
    if (!/pcm\.!default \{ type null \}/.test(code)) problems.push("starts a Chrome and never writes a null ALSA configuration");
    return problems;
}

test("every script under scripts/ that starts a Chrome has the silent form; cold-play.sh asks for it", () => {
    const dir = path.join(path.resolve(__dirname, "../../.."), "scripts");
    const bad = fs.readdirSync(dir).filter((name) => name.endsWith(".sh"))
        .flatMap((name) => scriptProblems(fs.readFileSync(path.join(dir, name), "utf8")).map((what) => `scripts/${name}: ${what}`));
    assert.deepEqual(bad, []);
    assert.match(fs.readFileSync(path.join(dir, "mp1-play.sh"), "utf8"), /"\$CHROME_BIN" "\$\{ARGS\[@\]\}"/, "the scan sees the Chrome start of mp1-play.sh");
    const cold = fs.readFileSync(path.join(dir, "cold-play.sh"), "utf8").split("\n").filter((line) => !/^\s*#/.test(line)).join("\n");
    assert.match(cold, /mp1-play\.sh" "\$\{play_args\[@\]\}" --silent/, "cold-play.sh starts the play script with --silent");
});

test("the script scan finds a Chrome start with no silent form, and passes one that has it", () => {
    assert.equal(scriptProblems('"$CHROME_BIN" "${ARGS[@]}" "$URL" &').length, 3);
    assert.equal(scriptProblems('google-chrome --headless "$@"').length, 3);
    assert.equal(scriptProblems('exec chromium --no-sandbox').length, 3);
    assert.deepEqual(scriptProblems('# "$CHROME_BIN" "${ARGS[@]}" in a comment\necho hello'), []);
    assert.deepEqual(scriptProblems('CHROME_BIN=x; node run.js'), [], "a script that only names the binary does not start it");
    const good = 'ARGS+=(--alsa-output-device=null)\nexport PULSE_SERVER="unix:/nonexistent/x"\nprintf \'pcm.!default { type null }\' > c\n"$CHROME_BIN" "${ARGS[@]}" "$URL" &';
    assert.deepEqual(scriptProblems(good), []);
    assert.equal(scriptProblems(good.replace("--alsa-output-device=null", "--other")).length, 1, "the argument alone is required");
});
