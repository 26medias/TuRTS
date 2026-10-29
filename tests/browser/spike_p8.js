// turts-test-role: instrument
// Network spike P8 (Task 6b): the wasm client joins the spike server in headless Chrome.
//
// Usage: node spike_p8.js <page url> <ws host:port>
// Launches CHROME_BIN (default /usr/bin/google-chrome) headless, opens <page url>?ws=<ws>,
// and waits up to 20 s for window.spike_state === "welcomed". Exit 0 on "welcomed".
// Otherwise (timeout or "rejected") it prints the console logs and exits 1.
const { chromium } = require("playwright-core");

const WAIT_MS = 20000;
const POLL_MS = 100;

(async () => {
    const [pageUrl, ws] = process.argv.slice(2);
    if (!pageUrl || !ws) {
        console.error("usage: node spike_p8.js <page url> <ws host:port>");
        process.exit(2);
    }
    const url = new URL(pageUrl);
    url.searchParams.set("ws", ws);
    const executablePath = process.env.CHROME_BIN || "/usr/bin/google-chrome";
    const browser = await chromium.launch({ executablePath, headless: true });
    const page = await browser.newPage();
    const logs = [];
    page.on("console", (m) => logs.push(m.type() + ": " + m.text()));
    page.on("pageerror", (e) => logs.push("pageerror: " + e.message));
    const t0 = Date.now();
    await page.goto(url.toString());
    let state = null;
    while (Date.now() - t0 < WAIT_MS) {
        state = await page.evaluate(() => window.spike_state || null).catch(() => null);
        if (state === "welcomed" || state === "rejected") {
            break;
        }
        await page.waitForTimeout(POLL_MS);
    }
    const ms = Date.now() - t0;
    console.log(JSON.stringify({ chrome: browser.version(), executablePath, url: url.toString(), state, ms }));
    await browser.close();
    if (state === "welcomed") {
        process.exit(0);
    }
    for (const line of logs) {
        console.log(line);
    }
    process.exit(1);
})().catch((e) => {
    console.error(e);
    process.exit(1);
});
