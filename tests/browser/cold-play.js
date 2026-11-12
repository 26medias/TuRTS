// turts-test-role: instrument
"use strict";

// Q-COLD: the cold-play step tool, the call that the agent makes for each input (docs/qa/cold-play.md).
//
//   node tests/browser/cold-play.js <session folder> <input>
//
// The session process (tests/browser/cold-session.js, started by scripts/cold-play.sh) holds the
// Chrome connection. This file sends one call to it over the loopback control port and prints the
// reply. It has no Playwright, reads no page and prints nothing that the session did not put in
// the reply: the input, the screenshot paths, the clocks and the refusal texts.
//
// Exit: 0 done; 2 the call is wrong (usage, a point outside the page); 3 refused by the rules
// (the statement is due, too early, or has problems); 4 the session has ended; 5 no session runs
// at that folder; 6 the tool could not do the input.
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");
const { USAGE } = require("./cold-session.js");

const MAX_STATEMENT_BYTES = 16384;
const CALLS = new Set(["move", "click", "drag", "wheel", "key", "wait", "status", "statement", "end"]);

// Ask the session. `timeoutMs` bounds the wait for the reply.
function ask(port, request, timeoutMs) {
    return new Promise((resolve, reject) => {
        const socket = net.connect({ host: "127.0.0.1", port });
        let buffer = "";
        const timer = setTimeout(() => { socket.destroy(); reject(new Error("the session gave no reply")); }, timeoutMs);
        socket.on("connect", () => socket.write(JSON.stringify(request) + "\n"));
        socket.on("data", (chunk) => { buffer += chunk; });
        socket.on("error", (error) => { clearTimeout(timer); reject(error); });
        socket.on("close", () => {
            clearTimeout(timer);
            try { resolve(JSON.parse(buffer.trim())); } catch { reject(new Error("the session gave no reply")); }
        });
    });
}

async function main(argv, out = process.stdout, err = process.stderr) {
    const [folder, cmd, ...args] = argv;
    if (!folder || !cmd || !CALLS.has(cmd)) {
        err.write(`usage: node tests/browser/cold-play.js <session folder> <call>\n${USAGE}\n`);
        return 2;
    }
    const request = { cmd, args };
    if (cmd === "statement") {
        if (args.length !== 1) { err.write("usage: statement FILE\n"); return 2; }
        let text;
        try {
            const size = fs.statSync(args[0]).size;
            if (size > MAX_STATEMENT_BYTES) throw new Error(`the file is larger than ${MAX_STATEMENT_BYTES} bytes`);
            text = fs.readFileSync(args[0], "utf8");
        } catch (error) { err.write(`cannot read the statement file: ${error.message}\n`); return 2; }
        request.text = text;
        request.args = [];
    }
    let control;
    try { control = JSON.parse(fs.readFileSync(path.join(folder, "internal", "control.json"), "utf8")); } catch {
        err.write("no session runs at that folder (it may have ended: see end.json in the folder)\n");
        return 5;
    }
    let reply;
    try { reply = await ask(control.port, request, 90000); } catch (error) {
        err.write(`no session runs at that folder (${error.message})\n`);
        return 5;
    }
    out.write(reply.lines.join("\n") + "\n");
    return reply.exit;
}

if (require.main === module) {
    main(process.argv.slice(2)).then((code) => { process.exitCode = code; });
}

module.exports = { main, ask };
