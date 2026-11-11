// turts-test-role: driver
"use strict";

const crypto = require("node:crypto");
const captureIndexes = new Map();

// Task16 physical driver. A guarded SLOT1 candidate can collect diagnostic
// artifacts, but only an unguarded executable plan can produce oracle evidence.
// Fixture setup stays in the harness runner.
async function drive(io, params, hooks = {}) {
    const plan = params.task16;
    const candidate = plan?.status === "candidate" &&
        params.instrument_status?.startsWith("SOURCE CANDIDATE ONLY");
    const executable = plan?.status === "executable" && params.instrument_status === undefined;
    if (!candidate && !executable) {
        throw new Error(`${params.row} has no runnable Task16 physical plan: ${params.instrument_status || "Task16 instrument guard"}`);
    }
    if (!plan || !Array.isArray(plan.actions) || !Array.isArray(plan.captures) || !Array.isArray(plan.checks) ||
        plan.actions.length === 0 || plan.captures.length === 0 || plan.checks.length === 0) {
        throw new Error(`${params.row} requires validated physical actions, captures, and row checks`);
    }
    const phase = params.fixture_phase || null;
    if (!phase && params.row !== "SLOT1" && Array.isArray(plan.phases) && plan.phases.length > 1) {
        captureIndexes.delete(params.row);
        for (const declaredPhase of plan.phases) await drive(io, { ...params, fixture_phase: declaredPhase });
        return;
    }
    const actions = plan.actions.filter((action) => !phase || action.phase === phase);
    const checkpoints = plan.captures.filter((capture) => !phase || capture.phase === phase);
    if (phase && (!actions.length || !checkpoints.length)) throw new Error(`${params.row} has no physical work for fixture phase ${phase}`);
    const indexName = `${params.row}-capture-index.json`;
    const captures = phase ? [...(captureIndexes.get(params.row) || [])] : [];
    const phaseCaptureStart = captures.length;
    const capture = async (checkpoint) => {
        await io.use(checkpoint.window);
        const screenshot = await io.screenshot();
        const bytes = Buffer.from(screenshot);
        const sha256 = crypto.createHash("sha256").update(bytes).digest("hex");
        const name = `${params.row}-${checkpoint.phase}-${checkpoint.name}.png`;
        await io.save(name, bytes);
        captures.push({ name: checkpoint.name, file: name, phase: checkpoint.phase,
            ordinal: captures.length, window: checkpoint.window, after_action: checkpoint.after_action,
            input_call: screenshot.input_call, document_epoch: screenshot.document_epoch,
            byte_length: bytes.length, sha256 });
    };
    let checkpointIndex = 0;
    const captureReady = async (completedActions) => {
        while (checkpointIndex < checkpoints.length && checkpoints[checkpointIndex].after_action === completedActions) {
            await capture(checkpoints[checkpointIndex]);
            checkpointIndex += 1;
        }
    };
    await captureReady(0);
    for (let actionIndex = 0; actionIndex < actions.length; actionIndex += 1) {
        const action = actions[actionIndex];
        if (typeof hooks.beforeAction === "function") await hooks.beforeAction(action, actionIndex, phase);
        switch (action.call) {
            case "key":
                await io.key(action.code, action.modifiers || [], action.options || {});
                break;
            case "click":
                requirePoint(action);
                await io.click(action.x, action.y, action.button || "left");
                break;
            case "drag":
                requirePoint(action);
                if (![action.x2, action.y2].every(Number.isFinite)) throw new Error("drag needs reviewed x2/y2 pixel coordinates");
                await io.drag(action.x, action.y, action.x2, action.y2, action.button || "left");
                break;
            case "move":
                requirePoint(action);
                await io.move(action.x, action.y);
                break;
            case "wheel":
                requirePoint(action);
                if (!Number.isFinite(action.delta)) throw new Error("wheel needs a finite delta");
                await io.move(action.x, action.y);
                await io.wheel(action.delta);
                break;
            case "wait":
                if (!Number.isInteger(action.ms) || action.ms < 0 || action.ms > 60000) throw new Error("wait needs ms in 0..60000");
                await io.wait(action.ms);
                break;
            case "reload":
                await io.reload();
                break;
            case "use":
                if (!Number.isInteger(action.window)) throw new Error("use needs a window number");
                await io.use(action.window);
                break;
            default:
                throw new Error(`${params.row} has unsupported harness action ${String(action.call)}`);
        }
        if (typeof hooks.afterAction === "function") await hooks.afterAction(action, actionIndex, phase);
        await captureReady(actionIndex + 1);
    }
    if (checkpointIndex !== checkpoints.length) throw new Error(`${params.row} capture after_action is outside its phase action list`);
    captureIndexes.set(params.row, captures);
    const phaseName = phase || plan.phases[0];
    await io.save(`${params.row}-capture-index-${phaseName}.json`, Buffer.from(JSON.stringify(captures.slice(phaseCaptureStart), null, 2)));
    if (!phase || phase === plan.phases.at(-1)) {
        await io.save(indexName, Buffer.from(JSON.stringify(captures, null, 2)));
    }
}

function requirePoint(action) {
    if (!Number.isFinite(action.x) || !Number.isFinite(action.y)) {
        throw new Error(`NOT TRACED: ${action.call} needs a reviewed screenshot pixel point in the row parameters`);
    }
}

module.exports = { drive };
