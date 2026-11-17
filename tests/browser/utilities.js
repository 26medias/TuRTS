// turts-test-role: instrument
// M6 UT8: real pointer and key input in a fresh silent browser. The bridges only find points
// and check results; every action is an ordinary mouse or key event.
// node tests/browser/utilities.js URL OUTPUT [CHROME]
'use strict';
const fs = require('node:fs'), os = require('node:os'), path = require('node:path');
const {spawn} = require('node:child_process'), {chromium} = require('playwright-core');
const {decodePng} = require('./harness/png');
const [url, output, executablePath = '/usr/bin/google-chrome'] = process.argv.slice(2);
if (!output) throw Error('URL OUTPUT [CHROME]');
fs.mkdirSync(output, {recursive: true});
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const assert = (value, message) => { if (!value) throw Error(message); };
// The utility_yard fixture: 2-unit cells, 0.5-unit elevation steps, ridge cells x 11..12 at 2.
const STEP = 0.5, CELL = 2;
const GENERATOR = 'utility_yard:generator', WORKSHOP = 'utility_yard:workshop';
const DEPOT = 'utility_yard:depot', PORTER = 'utility_yard:porter';
const LINE = 'utility_yard:power_line';
const DIG_CELL = {x: 11, y: 5};
function project(scene, [x, y, height = 0]) {
    const m = scene.clip_from_world, v = [x, height, y, 1];
    const c = [0, 1, 2, 3].map(r => v.reduce((a, n, k) => a + m[k * 4 + r] * n, 0));
    assert(c[3] > 0, 'Point behind camera');
    const [left, top, width, depth] = scene.viewport_logical_rect;
    return [left + (c[0] / c[3] + 1) * width / 2, top + (1 - c[1] / c[3]) * depth / 2];
}
function changedPixels(beforeBytes, afterBytes, [x0, y0, x1, y1]) {
    const a = decodePng(beforeBytes), b = decodePng(afterBytes);
    let count = 0;
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
        const i = (y * a.width + x) * 4;
        const change = Math.abs(a.rgba[i] - b.rgba[i]) + Math.abs(a.rgba[i + 1] - b.rgba[i + 1]) + Math.abs(a.rgba[i + 2] - b.rgba[i + 2]);
        if (change > 30) count++;
    }
    return count;
}
// Plan-view intersection of two segments, from their world end points [x, height, y].
function crosses(a, b) {
    const p = [a.a[0], a.a[2]], r = [a.b[0] - a.a[0], a.b[2] - a.a[2]];
    const q = [b.a[0], b.a[2]], s = [b.b[0] - b.a[0], b.b[2] - b.a[2]];
    const d = r[0] * s[1] - r[1] * s[0];
    if (Math.abs(d) < 1e-9) return false;
    const t = ((q[0] - p[0]) * s[1] - (q[1] - p[1]) * s[0]) / d, u = ((q[0] - p[0]) * r[1] - (q[1] - p[1]) * r[0]) / d;
    return t > 0.01 && t < 0.99 && u > 0.01 && u < 0.99;
}
(async () => {
    const result = {status: 'RUNNING', errors: [], actions: [], states: [], checks: {}};
    const save = () => fs.writeFileSync(path.join(output, 'result.json'), JSON.stringify(result, null, 2));
    let context, xvfb, profile;
    try {
        xvfb = spawn('Xvfb', ['-displayfd', '3', '-screen', '0', '1280x720x24', '-nolisten', 'tcp'], {stdio: ['ignore', 'ignore', 'pipe', 'pipe']});
        const display = await new Promise((resolve, reject) => { let value = ''; xvfb.stdio[3].on('data', data => { value += data; if (value.includes('\n')) resolve(':' + value.trim()); }); xvfb.once('error', reject); xvfb.once('exit', code => reject(Error('Xvfb exited ' + code))); });
        profile = fs.mkdtempSync(path.join(os.tmpdir(), 'turts-utilities-'));
        context = await chromium.launchPersistentContext(profile, {executablePath, headless: false, viewport: {width: 1280, height: 720}, env: {...process.env, DISPLAY: display}, args: ['--alsa-output-device=null', '--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--disable-vulkan-surface', '--no-first-run', '--no-default-browser-check']});
        result.chrome_args_include_null_audio = true;
        const page = context.pages()[0];
        page.on('console', message => fs.appendFileSync(path.join(output, 'console.log'), message.type() + ': ' + message.text() + '\n'));
        page.on('pageerror', error => result.errors.push(String(error)));
        await page.goto(url);
        await page.keyboard.press('F9');
        const read = () => page.evaluate(async () => {
            if (typeof window.turts_debug_request !== 'function' || typeof window.turts_scene !== 'function') return null;
            const raw = await window.turts_debug_request(), scene = window.turts_scene();
            return raw && scene ? {debug: JSON.parse(raw), scene: JSON.parse(scene), ui: typeof window.turts_ui === 'function' ? JSON.parse(window.turts_ui()) : []} : null;
        });
        const wait = async (predicate, message, timeout = 90000) => { const end = Date.now() + timeout; while (Date.now() < end) { const s = await read(); if (s && predicate(s)) return s; await sleep(75); } throw Error(message); };
        const capture = async name => { const s = await read(); fs.writeFileSync(path.join(output, name + '.json'), JSON.stringify(s, null, 2)); await page.screenshot({path: path.join(output, name + '.png')}); result.states.push(name); save(); console.log('UTILITY_CAPTURE ' + name); return s; };
        const world = s => s.debug.replicated_world, terrain = s => world(s).terrain, util = s => s.scene.utilities;
        const unit = (s, content) => world(s).units.find(u => u.owner_slot === s.debug.slot && u.content_id === content);
        const surface = (s, cell) => terrain(s).surfaces.find(r => r.cell.x === cell.x && r.cell.y === cell.y);
        const contact = (s, id) => { const anchor = terrain(s).anchors.find(a => a.unit === id && a.current); return anchor ? anchor.elevation * STEP : 0; };
        const ground = ([x, y]) => { const cx = Math.floor(x / CELL); return [x, y, (cx === 11 || cx === 12) ? 2 * STEP : 0]; };
        const click = async (point, button = 'left') => {
            assert(point.every(Number.isFinite) && point[0] > 0 && point[0] < 1280 && point[1] > 0 && point[1] < 720, 'Pointer target is outside viewport: ' + point);
            result.actions.push({kind: 'click', point, button});
            await page.mouse.move(...point); await sleep(80); await page.mouse.down({button}); await sleep(80); await page.mouse.up({button}); await sleep(120);
        };
        const key = async code => { result.actions.push({kind: 'key', code}); await page.keyboard.press(code); await sleep(150); };
        const select = async (content, height) => { const s = await read(), u = unit(s, content); assert(u, 'Owned unit missing: ' + content); await click(project(s.scene, [...u.pos, contact(s, u.unit_id) + height])); await wait(s => s.debug.selection?.includes(u.unit_id), 'Body click did not select ' + content, 5000); return u.unit_id; };
        const widget = async (predicate, what) => { const s = await wait(s => s.ui.some(w => w.visible && predicate(w)), 'Widget not visible: ' + what, 10000); const w = s.ui.find(w => w.visible && predicate(w)), [x, y, width, height] = w.rect; await click([x + width / 2, y + height / 2]); };
        const routeButton = () => widget(w => w.enabled && w.widget_action?.kind === 'utility_route' && w.widget_action.utility === LINE, 'Power line route button');
        const toolButton = tool => widget(w => w.enabled && w.widget_action?.kind === 'terrain_work' && w.widget_action.tool === tool, tool);
        const port = (s, id) => util(s).ports.find(p => p.unit === id);
        const lineHeights = s => util(s).segments.map(g => [g.id, g.a[1], g.b[1]]).sort((l, r) => l[0] - r[0]);

        let s = await wait(s => world(s)?.terrain && world(s).units.length && unit(s, GENERATOR) && unit(s, WORKSHOP), 'World did not activate');
        assert(s.debug.slot === 0, 'Joined the wrong slot');
        const generator = unit(s, GENERATOR).unit_id, workshop = unit(s, WORKSHOP).unit_id;
        await capture('00-start');
        assert(!util(s).view_on && util(s).segments.length === 0 && util(s).ports.length === 0, 'Utility view drew with the view off');
        // 1. View on: terrain fades and the two ports show as unconnected.
        const fadeBox = (() => { const p = project(s.scene, [10, 26, 0]); return [Math.round(p[0] - 40), Math.round(p[1] - 25), Math.round(p[0] + 40), Math.round(p[1] + 25)]; })();
        const before = await page.screenshot();
        await key('KeyU');
        s = await wait(s => util(s).view_on && util(s).ports.length >= 2, 'U did not turn the utility view on');
        await sleep(300);
        const after = await page.screenshot();
        fs.writeFileSync(path.join(output, '01-view-on.png'), after);
        const faded = changedPixels(before, after, fadeBox);
        assert(faded > 400, 'Terrain did not fade in the utility view: ' + faded);
        assert(port(s, generator)?.state === 'unconnected' && port(s, workshop)?.state === 'unconnected', 'Ports are not unconnected at start');
        result.checks.view_on = {faded_pixels: faded, ports: util(s).ports};
        // 2. Two lines that cross in plan view: generator to the ridge, workshop to the west.
        await select(DEPOT, 1.0);
        await routeButton();
        s = await wait(s => util(s).route_mode, 'Route button did not start route mode');
        await click(port(s, generator).screen);
        await click(project(s.scene, ground([25.5, 8])));
        s = await wait(s => util(s).segments.length === 1, 'First line was not placed');
        await key('Escape');
        await routeButton();
        s = await wait(s => util(s).route_mode, 'Route mode did not restart');
        await click(port(s, workshop).screen);
        await click(project(s.scene, ground([21, 8])));
        s = await wait(s => util(s).segments.length === 2 && port(s, workshop)?.state === 'short', 'Second line was not placed or the workshop is not short');
        assert(crosses(util(s).segments[0], util(s).segments[1]), 'The two lines do not cross in plan view');
        await sleep(500);
        s = await read();
        assert(port(s, workshop).state === 'short', 'A plan-view crossing connected the workshop');
        await capture('02-crossing-short');
        result.checks.crossing = {segments: util(s).segments, workshop: port(s, workshop)};
        // 3. An explicit route on the ridge at depth 1 joins the two lines through splits.
        await key('Escape');
        await routeButton();
        s = await wait(s => util(s).route_mode, 'Route mode did not restart for the junction');
        await key('BracketLeft');
        s = await wait(s => util(s).route?.depth === 1, '[ did not change the route depth to 1');
        await click(project(s.scene, ground([22.58, 11.2])));
        await click(project(s.scene, ground([24.44, 11.2])));
        s = await wait(s => util(s).segments.length === 5 && port(s, workshop)?.state === 'supplied', 'Explicit junction route did not supply the workshop');
        await key('Escape');
        const fixed = lineHeights(s);
        const shallow = util(s).segments.filter(g => Math.abs(g.a[1] - STEP) < 0.1 && Math.abs(g.b[1] - STEP) < 0.1);
        assert(shallow.length === 1, 'No depth-1 line on the ridge: ' + JSON.stringify(fixed));
        await page.mouse.move(...port(s, workshop).screen); await sleep(400);
        s = await wait(s => /Powered/.test(util(s).text || ''), 'Hover inspection does not show the Powered label', 5000);
        await capture('03-connected');
        result.checks.connected = {segments: util(s).segments, inspection: util(s).text};
        // 4. View off: normal terrain work digs the ridge cell down to the flat level.
        await key('KeyU');
        s = await wait(s => !util(s).view_on && util(s).segments.length === 0, 'U did not turn the view off');
        await select(PORTER, 0.6);
        await toolButton('utility_yard:scraper');
        s = await read();
        await click(project(s.scene, [19, 11, 0]));
        const digPoint = [(DIG_CELL.x + .5) * CELL, (DIG_CELL.y + .5) * CELL, 2 * STEP];
        s = await read();
        const dp = project(s.scene, digPoint);
        await page.mouse.move(...dp); await sleep(80); await page.mouse.down(); await sleep(150); await page.mouse.move(dp[0] + 1, dp[1] + 1); await sleep(80); await page.mouse.up(); await sleep(150);
        result.actions.push({kind: 'designate', cell: DIG_CELL, tool: 'scraper'});
        s = await wait(s => surface(s, DIG_CELL)?.current && surface(s, DIG_CELL).elevation === 0, 'Digging did not lower the ridge cell to 0', 180000);
        await key('KeyU');
        s = await wait(s => util(s).view_on && util(s).segments.some(g => g.state === 'disconnected' && g.dashed) && port(s, workshop)?.state === 'short', 'The exposed line did not disconnect the workshop');
        assert(JSON.stringify(lineHeights(s)) === JSON.stringify(fixed), 'Digging moved a line');
        await capture('04-exposed-disconnected');
        result.checks.exposed = {segments: util(s).segments, workshop: port(s, workshop)};
        // 5. View off: a paid fill raises the cell back to the ridge level; the line reconnects.
        const balance = s => world(s).economy.balances.find(b => b.resource === 'utility_yard:loam')?.on_hand;
        await key('KeyU');
        await select(PORTER, 0.6);
        await toolButton('utility_yard:leveler');
        s = await read();
        await click(project(s.scene, [25, 13, 2 * STEP]));
        s = await read();
        const beforeFill = balance(s);
        // The near ridge wall hides the south part of the pit floor from this camera.
        const fp = project(s.scene, [digPoint[0], DIG_CELL.y * CELL + 0.4, 0]);
        await page.mouse.move(...fp); await sleep(80); await page.mouse.down(); await sleep(150); await page.mouse.move(fp[0] + 1, fp[1] + 1); await sleep(80); await page.mouse.up(); await sleep(150);
        result.actions.push({kind: 'designate', cell: DIG_CELL, tool: 'leveler'});
        await sleep(1500);
        await capture('05a-fill-designated');
        s = await wait(s => surface(s, DIG_CELL)?.current && surface(s, DIG_CELL).elevation === 2, 'Fill did not raise the cell to 2', 180000);
        assert(balance(s) < beforeFill, 'Fill was not paid');
        await key('KeyU');
        s = await wait(s => util(s).view_on && util(s).segments.every(g => g.state === 'conducting') && port(s, workshop)?.state === 'supplied', 'Fill did not reconnect the workshop');
        assert(JSON.stringify(lineHeights(s)) === JSON.stringify(fixed), 'Filling moved a line');
        await capture('05-filled-reconnected');
        result.checks.refilled = {loam_before: beforeFill, loam_after: balance(s), segments: util(s).segments};
        // 6. Both zoom limits keep the lines on screen.
        const zooms = [];
        for (const [zoom, delta] of [['near', -10000], ['far', 10000]]) {
            s = await read();
            await page.mouse.move(...port(s, workshop).screen);
            await page.mouse.wheel(0, delta); await sleep(400); await page.mouse.wheel(0, delta); await sleep(400);
            s = await capture('06-zoom-' + zoom);
            const onScreen = util(s).segments.filter(g => [g.a_screen, g.b_screen].some(p => p && p[0] > 0 && p[0] < 1280 && p[1] > 0 && p[1] < 720));
            assert(onScreen.length > 0, 'No line on screen at the ' + zoom + ' zoom limit');
            zooms.push({zoom, matrix: s.scene.clip_from_world, on_screen: onScreen.length});
        }
        assert(Math.max(...zooms[0].matrix.map((n, i) => Math.abs(n - zooms[1].matrix[i]))) > .01, 'Zoom limits are identical');
        result.checks.zoom = zooms;
        // 7. A route that is too long shows a reason and sends nothing.
        await page.mouse.wheel(0, -1500); await sleep(400);
        await select(DEPOT, 1.0);
        await routeButton();
        s = await wait(s => util(s).route_mode, 'Route mode did not start for the refusal');
        const count = util(s).segments.length;
        await click(port(s, generator).screen);
        s = await read();
        // 16.8 world units from the generator port: over the 16-unit authored limit.
        const far = project(s.scene, [35, 17, 0]);
        await page.mouse.move(far[0] - 8, far[1] - 8); await sleep(150); await page.mouse.move(...far); await sleep(300);
        s = await wait(s => util(s).route?.reason, 'No reason for a too-long route', 5000);
        result.checks.refusal = {reason: util(s).route.reason};
        await click(far);
        await sleep(1000);
        s = await read();
        assert(util(s).segments.length === count, 'A too-long route was placed');
        await key('Escape');
        // 8. View off: normal selection and a move still work.
        await key('KeyU');
        s = await wait(s => !util(s).view_on, 'View did not turn off');
        const mover = await select(PORTER, 0.6);
        s = await read();
        const target = [16, 6];
        await click(project(s.scene, [...target, 0]), 'right');
        await wait(s => { const u = world(s).units.find(u => u.unit_id === mover); return u && Math.hypot(u.pos[0] - target[0], u.pos[1] - target[1]) < 1; }, 'Normal move did not arrive with the view off', 60000);
        await capture('07-normal-play');
        assert(result.errors.length === 0, 'Browser errors: ' + result.errors.join('; '));
        result.status = 'PASS';
    } catch (error) {
        result.status = 'FAIL';
        result.error = String(error.stack || error);
        if (context) {
            const page = context.pages()[0];
            await page.screenshot({path: path.join(output, 'failure.png')}).catch(() => {});
            result.failure_state = await page.evaluate(async () => ({scene: window.turts_scene?.(), ui: window.turts_ui?.(), debug: await window.turts_debug_request?.()})).catch(String);
        }
        process.exitCode = 1;
        console.error(error);
    } finally {
        if (context) await context.close();
        if (profile) fs.rmSync(profile, {recursive: true, force: true});
        if (xvfb && xvfb.exitCode === null) { const stopped = new Promise(resolve => xvfb.once('exit', resolve)); xvfb.kill('SIGTERM'); await Promise.race([stopped, sleep(2000)]); if (xvfb.exitCode === null && xvfb.signalCode === null) { xvfb.kill('SIGKILL'); await stopped; } }
        result.profile_removed = !profile || !fs.existsSync(profile);
        result.xvfb_stopped = !xvfb || xvfb.exitCode !== null || xvfb.signalCode !== null;
        save();
    }
})();
