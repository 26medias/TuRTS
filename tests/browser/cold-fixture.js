// turts-test-role: instrument
"use strict";

// Q-COLD pass 2: a small "game" page for the tests of the probes (cold-probe.test.js). It has what the real
// client has that the probes read: three own units, one enemy, a start overlay that takes the first click, an
// optional guide layer over everything (opaque or see-through, with a long text and a "Close" button), optional alert
// lines, an optional "Help" button, and the three bridges (turts_debug, turts_scene, turts_ui) with the fields that
// the probes use. A unit takes a left click and then selects (a ring and a panel) unless a layer is over it.
//
// Options (all optional):
//   startOverlay  true: "Click to play" over the whole page; it takes the first click and leaves
//   guide         null | "opaque" | "seethrough": a full-size layer, z-order under the overlay
//   guideWords    the number of words of the guide text (default 12)
//   alerts        the number of alert lines (and of debug alerts) that exist at the start
//   units         the number of own units (default 3)
//   selectShows   false: a click selects but draws nothing
//   selectsNothing true: a click draws the ring but selects nothing
//   helpButton    true: a "Help" button that opens a "help_panel" widget; helpDead true: it does nothing
//   alertsLateMs  the alert lines appear this long after the page loaded (default 0: at once)
//   hover         true: resting the pointer on the Help button shows a "help::tooltip" and a "help_glow" widget

function fixtureGame(options = {}) {
    const opts = { startOverlay: false, guide: null, guideWords: 12, alerts: 0, units: 3, selectShows: true, selectsNothing: false, helpButton: false, helpDead: false, hover: false, alertsLateMs: 0, ...options };
    return `<!doctype html><meta charset="utf-8"><title>fixture game</title>
<body style="margin:0;background:#1c1f26;font:16px sans-serif;color:#fff;overflow:hidden">
<style>
  .abs { position: absolute; box-sizing: border-box; }
  .unit { width: 60px; height: 60px; background: #1e64b4; }
  .enemy { background: #c87820; }
  #ring { border: 5px solid #ffe000; width: 84px; height: 84px; display: none; pointer-events: none; }
  #selection_panel { left: 0; top: 0; width: 260px; height: 260px; background: #223; padding: 8px; display: none; }
  #overlay { left: 0; top: 0; width: 1280px; height: 720px; background: rgba(0,0,0,0.3); z-index: 100; }
  #guide { left: 0; top: 0; width: 1280px; height: 720px; z-index: 50; }
  #guide_copy { left: 37px; top: 37px; width: 430px; height: 640px; overflow: hidden; }
  #guide_close { left: 1179px; top: 15px; width: 92px; height: 44px; background: #2a4; color: #fff; border: 0; }
  .alert { left: 929px; width: 332px; height: 28px; background: #234; z-index: 60; }
  #help { left: 1100px; top: 20px; width: 80px; height: 30px; z-index: 10; }
  #help_panel { left: 900px; top: 60px; width: 300px; height: 100px; background: #532; display: none; z-index: 10; padding: 8px; }
</style>
<div id="map" class="abs" style="left:200px;top:150px;width:880px;height:420px;background:#3b7a3b"></div>
<div class="abs" style="left:260px;top:450px;width:300px;height:100px;background:#2f5f9f"></div>
<div class="abs" style="left:700px;top:170px;width:240px;height:90px;background:#7a5f3b"></div>
<div id="units"></div>
<div id="ring" class="abs"></div>
<div id="selection_panel" class="abs"></div>
<div id="alerts"></div>
${opts.helpButton ? '<button id="help" class="abs">Help</button><div id="help_panel" class="abs">Help text</div>' : ""}
${opts.guide ? `<div id="guide" class="abs" style="background:${opts.guide === "opaque" ? "rgb(20,30,40)" : "rgba(20,30,40,0.8)"}"><div id="guide_copy" class="abs"></div><button id="guide_close">Close</button></div>` : ""}
${opts.startOverlay ? '<div id="overlay" class="abs"><div id="start_text" class="abs" style="left:546px;top:346px;width:188px;height:29px;font-size:26px">Click to play</div></div>' : ""}
<script>
const OPTS = ${JSON.stringify(opts)};
const OWN = [[1, 360, 300], [2, 640, 430], [3, 900, 300]].slice(0, OPTS.units);
const ENEMY = [9, 1000, 520];
let selection = [];
let selectedName = "";
const units = [...OWN.map(([id, x, y]) => ({ id, x, y, owner: 0 })), { id: ENEMY[0], x: ENEMY[1], y: ENEMY[2], owner: 1 }];
for (const unit of units) {
    const el = document.createElement("div");
    el.className = "abs unit" + (unit.owner ? " enemy" : "");
    el.style.left = (unit.x - 30) + "px"; el.style.top = (unit.y - 30) + "px";
    el.addEventListener("mousedown", (event) => {
        if (event.button !== 0 || unit.owner !== 0) return;
        if (!OPTS.selectsNothing) selection = [unit.id];
        if (OPTS.selectShows) {
            const ring = document.getElementById("ring");
            ring.style.display = "block"; ring.style.left = (unit.x - 42) + "px"; ring.style.top = (unit.y - 42) + "px";
            const panel = document.getElementById("selection_panel");
            panel.style.display = "block"; panel.textContent = "Selected: own" + unit.id; selectedName = panel.textContent;
        }
    });
    document.getElementById("units").appendChild(el);
}
const alertTexts = [];
function addAlerts() {
    for (let k = 0; k < OPTS.alerts; k += 1) {
        const text = "Alert " + k + ": something is waiting";
        alertTexts.push(text);
        const el = document.createElement("div");
        el.className = "abs alert"; el.style.top = (38 + 31 * k) + "px"; el.textContent = text;
        document.getElementById("alerts").appendChild(el);
    }
}
if (OPTS.alertsLateMs > 0) setTimeout(addAlerts, OPTS.alertsLateMs); else addAlerts();
const copy = document.getElementById("guide_copy");
if (copy) copy.textContent = Array.from({ length: OPTS.guideWords }, (_, k) => "word" + k).join(" ");
const overlay = document.getElementById("overlay");
if (overlay) overlay.addEventListener("mousedown", () => overlay.remove());
const guide = document.getElementById("guide");
if (guide) document.getElementById("guide_close").addEventListener("mousedown", () => guide.remove());
const help = document.getElementById("help");
let hovered = false;
if (help) { help.addEventListener("mouseenter", () => { hovered = OPTS.hover; }); help.addEventListener("mouseleave", () => { hovered = false; }); }
if (help && !OPTS.helpDead) help.addEventListener("mousedown", () => { const panel = document.getElementById("help_panel"); panel.style.display = panel.style.display === "block" ? "none" : "block"; });
const rect = (el) => { const r = el.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; };
const present = (id) => !!document.getElementById(id) && document.body.contains(document.getElementById(id));
function widgets() {
    const list = [];
    const w = (id, kind, el, extra = {}) => list.push({ id, kind, rect: rect(el), visible: true, enabled: true, capture_root: true, text: null, ...extra });
    const ov = document.getElementById("overlay");
    if (ov && document.body.contains(ov)) list.push({ id: "focus_overlay", kind: "focus_overlay", rect: [0, 0, 1280, 720], visible: true, enabled: true, capture_root: true, focus_policy_block: true,
        text_nodes: [{ rect: rect(document.getElementById("start_text")), text: "Click to play", visible: true }] });
    const g = document.getElementById("guide");
    if (g && document.body.contains(g)) { w("guide", "widget", g); w("guide_copy", "widget", copy, { text: copy.textContent }); w("guide_close", "button", document.getElementById("guide_close"), { text: "Close" }); }
    document.querySelectorAll(".alert").forEach((el, k) => w("alert_rows/alert_line[" + k + "]", "button", el, { text: el.textContent }));
    if (help) { w("help", "button", help, { text: "Help" }); const p = document.getElementById("help_panel"); if (p.style.display === "block") w("help_panel", "widget", p, { text: "Help text" }); }
    if (hovered) { list.push({ id: "help::tooltip", kind: "widget", rect: [1100, 52, 90, 20], visible: true, enabled: true, text: "Help tip" }); list.push({ id: "help_glow", kind: "widget", rect: [1096, 16, 88, 38], visible: true, enabled: true, text: null }); }
    const sp = document.getElementById("selection_panel");
    if (sp.style.display === "block") w("selection_panel", "widget", sp, { text: sp.textContent });
    return list;
}
const debug = () => JSON.stringify({ phase: "Joined { slot: 0 }", slot: 0, selection, alerts: alertTexts.map((text) => ({ text })),
    audio: { context_state: "running", sink_observations: [] },
    replicated_world: { units: units.map((u) => ({ unit_id: u.id, owner_slot: u.owner, content_id: "fixture:" + (u.owner ? "enemy" : "own" + u.id) })) } });
window.turts_debug = debug;
window.turts_debug_request = () => Promise.resolve(debug());
window.turts_scene = () => JSON.stringify({ units: units.map((u) => ({ unit_id: u.id, translation: [u.x, 0, u.y], ground_screen_center: [u.x, u.y], pick_radius: 30 })), nodes: [] });
window.turts_ui = () => JSON.stringify(widgets());
</script>`;
}

module.exports = { fixtureGame };
