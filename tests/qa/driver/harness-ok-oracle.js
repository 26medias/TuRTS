// turts-test-role: oracle
"use strict";

exports.judge = async (view, params) => {
    if (params.windows !== 2) throw new Error(`frozen params.windows was ${params.windows}, expected 2`);
    const secondDebug = await view.bridge("turts_debug", 2);
    if (!String(secondDebug.phase).startsWith("Joined")) throw new Error("view could not inspect the second joined window");
    const screenshot = await view.screenshot(1);
    const width = screenshot.readUInt32BE(16);
    const height = screenshot.readUInt32BE(20);
    if (width !== params.viewport.width || height !== params.viewport.height) throw new Error(`CSS screenshot dimensions were ${width}x${height}`);

    const calls = view.inputLog();
    const expected = ["click", "drag", "key", "move", "wheel", "screenshot", "save", "wait", "reload", "use", "use", "key", "use", "key", "key", "key", "key", "use", "key", "key", "key", "key", "key", "use", "click", "key", "key", "drag", "key"];
    if (calls.map((entry) => entry.call).join(",") !== expected.join(",")) throw new Error("input log lost call order");
    const expectedSeq = [
        "1,2,3", "4,5,6,7,8,9,10,11,12,13,14", "15,16", "17", "18", "", "", "", "", "", "", "1,2", "",
        "19", "20,21", "22", "23", "", "3,4", "5", "6,7,8,9", "10,11", "12", "", "24,25,26", "27", "28",
        "29,30,31,32,33,34,35,36,37,38,39", "40",
    ];
    if (calls.map((entry) => entry.event_seq.join(",")).join(";") !== expectedSeq.join(";")) throw new Error("input event sequences differ");
    if (calls.some((entry) => !Number.isFinite(entry.wall_time_ms) || !Array.isArray(entry.args))) throw new Error("input log lacks call arguments or wall times");
    for (const call of calls) {
        if (!Array.isArray(call.actual_events) || call.actual_events.map((event) => event.seq).join(",") !== call.event_seq.join(",")) throw new Error(`actual DOM sequence differs for ${call.call}`);
        if (call.actual_events.map((event) => event.type).join(",") !== call.event_types.join(",")) throw new Error(`actual DOM event types differ for ${call.call}`);
        const delivered = (call.downstream_events || []).filter((event) => Number.isInteger(event.seq));
        if (delivered.map((event) => event.seq).join(",") !== call.event_seq.join(",")) throw new Error(`game-target DOM sequence differs for ${call.call}`);
        if (delivered.map((event) => event.type).join(",") !== call.event_types.join(",")) throw new Error(`game-target DOM types differ for ${call.call}`);
        if ((call.downstream_events || []).some((event) => event.seq !== null && !Number.isInteger(event.seq))) throw new Error(`invalid game-target sequence for ${call.call}`);
    }
    if (calls[0].actual_events[1].client_x !== 20 || calls[0].actual_events[1].client_y !== 30) throw new Error("click reached the wrong DOM coordinates");
    if (calls[1].actual_events.at(-1).client_x !== 40 || calls[1].actual_events.at(-1).client_y !== 40) throw new Error("drag did not reach its requested endpoint");
    if (calls[2].actual_events[0].code !== "KeyA" || calls[2].actual_events[0].key !== "q") throw new Error("opts.key did not keep physical code and logical key separate");
    if (calls[4].actual_events[0].delta_y !== 90) throw new Error("wheel delta did not reach the DOM event");
    if (calls[11].window !== 2 || calls[11].actual_events[0].seq !== 1) throw new Error("second window event sequence did not start at one");

    const shiftHeld = (call) => call.downstream_events.filter((event) => Number.isInteger(event.seq)).map((event) => event.shift_key);
    if (shiftHeld(calls[14]).some((value) => !value) || shiftHeld(calls[15]).some((value) => !value) || shiftHeld(calls[16]).some((value) => !value)) {
        throw new Error("held Shift was lost during press or down/up logical-key override");
    }
    if (shiftHeld(calls[18]).some(Boolean)) throw new Error("window 2 inherited window 1's held Shift");
    if (calls[20].downstream_events.map((event) => event.shift_key).join(",") !== "true,true,true,false" || calls[20].actual_events.some((event) => !event.ctrl_key)) {
        throw new Error("temporary Shift did not compose with held Control and release only itself");
    }
    if (calls[21].actual_events.some((event) => !event.ctrl_key || event.shift_key)) throw new Error("temporary Shift leaked or held Control was lost");
    if (calls[22].actual_events.some((event) => event.ctrl_key || event.shift_key)) throw new Error("modifier release did not restore an unmodified state");
    if (shiftHeld(calls[24]).some((value) => !value) || shiftHeld(calls[27]).some((value) => !value)) throw new Error("Shift-held click or drag lost its modifier");
    if (shiftHeld(calls[25]).some(Boolean) || shiftHeld(calls[28]).some(Boolean)) throw new Error("Shift remained active after release");

    const args = [
        calls[0].args.join(",") === "20,30,left",
        calls[1].args.join(",") === "10,10,40,40,left",
        calls[2].args[0] === "KeyA" && calls[2].args[2].key === "q",
        calls[3].args.join(",") === "50,50",
        calls[4].args.join(",") === "90",
        calls[6].args[0] === "harness-shot.png" && calls[6].args[1].byte_length > 0 && /^[a-f0-9]{64}$/.test(calls[6].args[1].sha256),
        calls[7].args.join(",") === "10",
        calls[11].args[0] === "KeyB",
        calls[14].args[0] === "KeyA" && calls[14].args[1][0] === "Shift" && calls[14].args[2].key === "q",
        calls[15].args[2].action === "down" && calls[16].args[2].action === "up",
        calls[19].args[0] === "Control" && calls[19].args[2].action === "down",
        calls[20].args[1][0] === "Shift" && calls[20].args[2].key === "q",
        calls[22].args[0] === "Control" && calls[22].args[2].action === "up",
        calls[24].args.join(",") === "70,70,left",
        calls[27].args.join(",") === "60,60,90,90,left",
    ];
    if (args.some((ok) => !ok)) throw new Error("input log lost arguments");
    let refused = false;
    try { await view.bridge("turts_move"); } catch { refused = true; }
    if (!refused) throw new Error("view.bridge allowed turts_move");
};
