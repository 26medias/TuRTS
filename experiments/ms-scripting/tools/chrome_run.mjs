// Launch a fresh headless Chrome for one MS page run and write its JSON result.
// Usage: node tools/chrome_run.mjs <wasm name> <suite|probes> <kind> <run> <out.json> [mutant code]
// Serves web/ and out/wasm/ with COOP and COEP headers (cross-origin isolation, 5 us timer).
// Each launch uses a new temporary profile with --alsa-output-device=null; the profile is
// deleted at the end.
import { createRequire } from 'node:module';
import http from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire('/home/julien/Projects/TuRTS/tests/browser/package.json');
const { chromium } = require('playwright-core');

const [wasm, mode, kind, run, outPath, mutant = '0', only = ''] = process.argv.slice(2);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };

const server = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    if (u.pathname === '/favicon.ico') { res.writeHead(204); res.end(); return; }
    let file = u.pathname === '/' ? '/index.html' : u.pathname;
    const p = file.endsWith('.wasm') ? path.join(root, 'out/wasm', path.basename(file)) : path.join(root, 'web', path.basename(file));
    fs.readFile(p, (err, data) => {
        if (err) { res.writeHead(404); res.end(); return; }
        res.writeHead(200, {
            'Content-Type': types[path.extname(p)] || 'application/octet-stream',
            'Cross-Origin-Opener-Policy': 'same-origin',
            'Cross-Origin-Embedder-Policy': 'require-corp',
            'Cache-Control': 'no-store',
        });
        res.end(data);
    });
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const port = server.address().port;
const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'ms-chrome-'));
const args = ['--alsa-output-device=null', '--mute-audio', '--no-first-run', '--no-default-browser-check',
    '--disable-background-timer-throttling', '--disable-renderer-backgrounding'];
let ctx;
const consoleMsgs = [];
try {
    ctx = await chromium.launchPersistentContext(profile, {
        executablePath: '/usr/bin/google-chrome', headless: true, args,
    });
    const page = ctx.pages()[0] || await ctx.newPage();
    page.on('console', m => consoleMsgs.push(m.text()));
    const version = ctx.browser() ? ctx.browser().version() : (await page.evaluate(() => navigator.userAgent));
    await page.goto(`http://127.0.0.1:${port}/index.html?wasm=${wasm}&mode=${mode}&kind=${kind}&run=${run}&mutant=${mutant}${only ? '&only=' + only : ''}`);
    await page.waitForFunction(() => window.__done === true, null, { timeout: 900000, polling: 500 });
    const result = await page.evaluate(() => window.__result);
    const wrapped = `{"chrome":${JSON.stringify(version)},"args":${JSON.stringify(args)},"console_messages":${JSON.stringify(consoleMsgs)},"wasm":${JSON.stringify(wasm)},"data":${result}}`;
    fs.writeFileSync(outPath, wrapped);
} finally {
    if (ctx) await ctx.close();
    server.close();
    fs.rmSync(profile, { recursive: true, force: true });
}
