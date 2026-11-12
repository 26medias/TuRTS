// turts-test-role: unit
"use strict";

// Playwright does not await the predicate of `page.waitForFunction` (playwright-core 1.64.1): a
// Promise result is true at once. L0-DBG made four async predicates and the join gate stopped
// waiting. Every wait now goes through ONE helper, `harness/wait-sync.js`, which refuses an async
// predicate and throws when a predicate returns a value with `.then`. This test keeps it that way:
// the token `waitForFunction` may occur in no source file of the browser tests or the scripts, in
// any spelling that a scan can find, except in that helper. The scan removes comments, then removes
// quotes, `+` and white space, so `page["waitForFunction"]`, `page['wait' + 'ForFunction']`,
// `.waitForFunction.bind(page)` and `const { waitForFunction } = page` are all found.
// Known gap (as for the driver rule, A7): a key that a variable builds passes the scan. The helper
// still stops an async or Promise predicate that reaches it, and review must catch the variable key.
// Files `*.test.js` are not scanned: they define the fake pages that call the helper.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const REPO = path.resolve(__dirname, "../../..");
const HELPER = path.join(REPO, "tests/browser/harness/wait-sync.js");

// The source without comments. Strings and template literals are kept as they are.
function withoutComments(source) {
    let out = "";
    for (let i = 0; i < source.length; i += 1) {
        const char = source[i];
        if (char === "'" || char === '"' || char === "`") {
            let end = i + 1;
            for (; end < source.length && source[end] !== char; end += 1) if (source[end] === "\\") end += 1;
            out += source.slice(i, end + 1);
            i = end;
        } else if (char === "/" && source[i + 1] === "/") {
            while (i < source.length && source[i] !== "\n") i += 1;
            out += "\n";
        } else if (char === "/" && source[i + 1] === "*") {
            const end = source.indexOf("*/", i + 2);
            out += source.slice(i, end < 0 ? source.length : end + 2).replace(/[^\n]/g, " ");
            i = end < 0 ? source.length : end + 1;
        } else out += char;
    }
    return out;
}

// The lines of a source that name the Playwright wait, in any spelling that the scan can find.
function waitLines(source) {
    const lines = withoutComments(source).split("\n");
    return lines.flatMap((line, index) => (/waitForFunction/.test(line.replace(/[\s"'`+]/g, "")) ? [index + 1] : []));
}

function sourceFiles(directory, pattern) {
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        if (entry.name === "node_modules" || entry.name === "target") return [];
        const full = path.join(directory, entry.name);
        if (entry.isDirectory()) return sourceFiles(full, pattern);
        return pattern.test(entry.name) && !/\.test\.js$/.test(entry.name) && full !== HELPER ? [full] : [];
    });
}

test("no source file of tests/browser or scripts names waitForFunction outside harness/wait-sync.js", () => {
    const files = [...sourceFiles(path.join(REPO, "tests/browser"), /\.(?:js|mjs|cjs)$/),
        ...sourceFiles(path.join(REPO, "scripts"), /\.(?:js|mjs|cjs|sh|py)$/)];
    assert.ok(files.length > 50, "the scan reads the tree");
    const hits = files.flatMap((file) => waitLines(fs.readFileSync(file, "utf8")).map((line) => `${path.relative(REPO, file)}:${line}`));
    assert.deepEqual(hits, []);
});

test("the scan finds each form that the re-check planted, and every other spelling", () => {
    const planted = {
        "an object method": "const preds = { async joined() { return true; } };\nawait page.waitForFunction(preds.joined);",
        "a factory call": "const make = () => async () => true;\nawait page.waitForFunction(make());",
        "a Promise with no async": "await page.waitForFunction(() => Promise.resolve(true));",
        "the bracket key": 'await page["waitForFunction"](async () => true);',
        "the bracket key with a built string": "await page['wait' + 'ForFunction'](() => true);",
        "a bind alias": "const wait = page.waitForFunction.bind(page);\nawait wait(async () => true);",
        "a destructured alias": "const { waitForFunction } = page;",
        "a template key": "await page[`waitForFunction`](() => true);",
        "a frame call on a new line": "await frame\n    .waitForFunction(() => true);",
    };
    for (const [name, source] of Object.entries(planted)) assert.ok(waitLines(source).length > 0, `the scan must find ${name}`);
});

test("the scan ignores comments and finds the line", () => {
    assert.deepEqual(waitLines("// page.waitForFunction(x)\n/* waitForFunction\n waitForFunction */\nconst a = 1;"), []);
    assert.deepEqual(waitLines("const a = 'http://x';\n\nawait page.waitForFunction(f);"), [3], "a // inside a string is not a comment");
    assert.deepEqual(waitLines("await waitForSync(page, f);"), []);
});
