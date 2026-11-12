// turts-test-role: unit
"use strict";

// Playwright does not await the predicate of `page.waitForFunction` (playwright-core 1.64.1,
// coreBundle.js, the predicate result is tested for truth as it is): an `async` predicate returns
// a Promise, a Promise is true, and the wait passes at once. L0-DBG made four such waits and the
// join gate stopped waiting (review-l0-dbg.md, finding 1). This test reads every browser test file
// and refuses the three forms that let a Promise reach the result of a predicate.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const ROOT = path.resolve(__dirname, "..");

// The text of the first argument of each `waitForFunction(` call, with its line number. The scan
// skips strings, template literals and comments, so a paren in a string does not end the call.
function predicateArguments(source) {
    const found = [];
    const marker = /\bwaitForFunction\s*\(/g;
    let match;
    while ((match = marker.exec(source)) !== null) {
        const start = match.index + match[0].length;
        let depth = 1;
        let end = start;
        let firstEnd = null;
        for (; end < source.length && depth > 0; end += 1) {
            const char = source[end];
            if (char === "'" || char === '"' || char === "`") {
                for (end += 1; end < source.length && source[end] !== char; end += 1) if (source[end] === "\\") end += 1;
            } else if (char === "/" && source[end + 1] === "/") {
                while (end < source.length && source[end] !== "\n") end += 1;
            } else if (char === "/" && source[end + 1] === "*") {
                end = source.indexOf("*/", end + 2) + 1;
                if (end === 0) end = source.length;
            } else if (char === "(" || char === "[" || char === "{") depth += 1;
            else if (char === ")" || char === "]" || char === "}") depth -= 1;
            else if (char === "," && depth === 1 && firstEnd === null) firstEnd = end;
        }
        found.push({ line: source.slice(0, match.index).split("\n").length, text: source.slice(start, firstEnd ?? end - 1) });
    }
    return found;
}

// The defects in one source text: each is `{ line, rule }`.
function scanSource(source) {
    const defects = [];
    for (const { line, text } of predicateArguments(source)) {
        const body = text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "").trim();
        if (/^async\b/.test(body)) defects.push({ line, rule: "async predicate" });
        else if (/\bawait\b/.test(body)) defects.push({ line, rule: "await in a predicate" });
        const named = /^[A-Za-z_$][\w$]*$/.exec(body)?.[0];
        if (named && new RegExp(`(?:async\\s+function\\s+${named.replace(/\$/g, "\\$")}\\b|\\b${named.replace(/\$/g, "\\$")}\\s*=\\s*async\\b)`).test(source)) {
            defects.push({ line, rule: "named async predicate" });
        }
        // The promise of a request must never be the result: it is started and its failure is caught.
        for (const request of body.matchAll(/turts_debug_request\s*(?:\?\.)?\s*\(/g)) {
            const rest = body.slice(request.index + request[0].length);
            const close = rest.indexOf(")");
            if (!/^\s*\.catch\s*\(/.test(rest.slice(close + 1))) defects.push({ line, rule: "request promise is not caught" });
        }
    }
    return defects;
}

function browserTestFiles(directory = ROOT) {
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        if (entry.name === "node_modules") return [];
        const full = path.join(directory, entry.name);
        if (entry.isDirectory()) return browserTestFiles(full);
        // This file holds planted sources as strings, so the scan does not read it.
        return /\.(?:js|mjs|cjs)$/.test(entry.name) && full !== __filename ? [full] : [];
    });
}

test("no browser test file passes an async predicate to waitForFunction", () => {
    const files = browserTestFiles();
    assert.ok(files.length > 50, "the scan reads the tree of tests/browser");
    const defects = files.flatMap((file) => scanSource(fs.readFileSync(file, "utf8"))
        .map(({ line, rule }) => `${path.relative(ROOT, file)}:${line} ${rule}`));
    assert.deepEqual(defects, []);
});

test("the scan refuses each planted form and passes the sync form", () => {
    const planted = {
        "an async arrow": "await page.waitForFunction(async () => true, null, {});",
        "an async arrow, new line": "await page.waitForFunction(\n    async (x) => x,\n    1);",
        "an async arrow after a comment": "page.waitForFunction(/* why */ async () => true);",
        "await in a string expression": "page.waitForFunction('await window.turts_debug_request()');",
        "a named async function": "const ready = async () => true;\npage.waitForFunction(ready);",
        "an async function declaration": "async function ready() { return true; }\npage.waitForFunction(ready, null, {});",
        "the request promise as result": "page.waitForFunction(() => window.turts_debug_request());",
        "the request promise with no catch": "page.waitForFunction(() => { window.turts_debug_request?.(); return true; });",
    };
    for (const [name, source] of Object.entries(planted)) {
        assert.ok(scanSource(source).length > 0, `the scan must refuse ${name}`);
    }
    const good = [
        "page.waitForFunction(() => window.turts_debug() !== '', null, { timeout: 1 });",
        "page.waitForFunction((x) => { window.turts_debug_request?.().catch(() => {}); return x; }, 1);",
        "page.waitForFunction(() => window.__held === true,\n    null, { timeout: 30000, polling: 20 });",
        "const ready = () => true;\npage.waitForFunction(ready);",
        "page.waitForFunction(() => document.title.includes('(a)'));  // async () => nothing",
        "page.waitForFunction(() => new RegExp('a(b').test('x') || true);",
    ];
    for (const source of good) assert.deepEqual(scanSource(source), [], source);
});

test("the scan finds the line of a defect", () => {
    const source = "const a = 1;\n\nawait page.waitForFunction(async () => 1);\n";
    assert.deepEqual(scanSource(source), [{ line: 3, rule: "async predicate" }]);
});
