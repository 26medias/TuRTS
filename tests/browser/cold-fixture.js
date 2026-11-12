// turts-test-role: instrument
"use strict";

// A small "game" page for the tests of the probes (cold-probe.test.js). It has what the real client has that the
// probes read: three own units, one enemy, one resource node, a start overlay that takes the first click, an optional
// guide layer over everything (opaque or see-through, with a long text and a "Close" button), optional alert lines,
// an optional "Help" button, order buttons for a selected own unit, and the three bridges (turts_debug, turts_scene,
// turts_ui) with the fields that the probes use.
//
// Options (all optional):
//   startOverlay  true: "Click to play" over the whole page; it takes the first click and leaves
//   guide         null | "opaque" | "seethrough": a full-size layer, z-order under the overlay
//   guideWords    the number of words of the guide text (default 12)
//   alerts        the number of alert lines (and of debug alerts) that exist at the start
//   alertsLateMs  the alert lines appear this long after the page loaded (default 0: at once)
//   units         the number of own units (default 3)
//   selectShows   false: a click selects but draws nothing
//   noPanel       true: a selection draws the ring only (about 1,600 pixels), no name panel
//   selectsNothing true: a click draws the ring but selects nothing
//   noise         true: 100 ms after every mouse press a big block and an alert line appear by themselves
//   orders        false: a selected own unit shows no order button
//   ordersDead    true: the "Disable" order shows no result (the "Hold" order still shows a notice)
//   enemyShows    false: a click on the enemy shows no panel
//   nodeShows     false: a click on the node shows no name
//   groundKeeps   true: a click on open ground keeps the selection
//   rightDead     true: a right click on open ground shows nothing
//   cameraButton  true: a "Center" button that moves the camera: the scene centres shift by 40 px, nothing else changes
//   helpButton    true: a "Help" button that opens a "help_panel" widget; helpDead true: it does nothing
//   hover         true: resting the pointer on the Help button shows a "help::tooltip" and a "help_glow" widget

function fixtureGame(options = {}) {
    const opts = { startOverlay: false, guide: null, guideWords: 12, alerts: 0, alertsLateMs: 0, units: 3, selectShows: true, noPanel: false, selectsNothing: false, noise: false,
        orders: true, ordersDead: false, enemyShows: true, nodeShows: true, groundKeeps: false, rightDead: false, helpButton: false, helpDead: false, cameraButton: false, hover: false, ...options };
    return `<!doctype html><meta charset="utf-8"><title>fixture game</title>
<body style="margin:0;background:#1c1f26;font:16px sans-serif;color:#fff;overflow:hidden">
<style>
  .abs { position: absolute; box-sizing: border-box; }
  .unit { width: 60px; height: 60px; background: #1e64b4; }
  .enemy { background: #c87820; }
  .node { width: 50px; height: 50px; background: #d4b020; }
  #ring { border: 5px solid #ffe000; width: 84px; height: 84px; display: none; pointer-events: none; }
  #selection_panel { left: 0; top: 0; width: 260px; height: 260px; background: #223; padding: 8px; display: none; }
  #node_panel { left: 0; top: 270px; width: 260px; height: 80px; background: #342; padding: 8px; display: none; }
  #notice { left: 300px; top: 650px; width: 400px; height: 40px; background: #432; padding: 8px; display: none; }
  #mark { width: 30px; height: 30px; background: #0f0; display: none; pointer-events: none; }
  #state { left: 0; top: 360px; width: 260px; height: 40px; background: #522; padding: 8px; display: none; }
  #overlay { left: 0; top: 0; width: 1280px; height: 720px; background: rgba(0,0,0,0.3); z-index: 100; }
  #guide { left: 0; top: 0; width: 1280px; height: 720px; z-index: 50; }
  #guide_copy { left: 37px; top: 37px; width: 430px; height: 640px; overflow: hidden; }
  #guide_close { left: 1179px; top: 15px; width: 92px; height: 44px; background: #2a4; color: #fff; border: 0; }
  .alert { left: 929px; width: 332px; height: 28px; background: #234; z-index: 60; }
  #help { left: 1100px; top: 20px; width: 80px; height: 30px; z-index: 10; }
  #help_panel { left: 900px; top: 60px; width: 300px; height: 100px; background: #532; display: none; z-index: 10; padding: 8px; }
  .order { left: 1000px; width: 264px; height: 28px; background: #000; color: #fff; border: 0; z-index: 5; display: none; text-align: left; }
  #noise { left: 800px; top: 560px; width: 400px; height: 100px; background: #a0a; display: none; }
</style>
<div id="map" class="abs" style="left:200px;top:150px;width:880px;height:420px;background:#3b7a3b"></div>
<div class="abs" style="left:260px;top:450px;width:300px;height:100px;background:#2f5f9f"></div>
<div class="abs" style="left:700px;top:170px;width:240px;height:90px;background:#7a5f3b"></div>
<div id="units"></div>
<div id="node" class="abs node" style="left:535px;top:495px"></div>
<div id="ring" class="abs"></div>
<div id="mark" class="abs"></div>
<div id="selection_panel" class="abs"></div>
<div id="node_panel" class="abs"></div>
<div id="state" class="abs">off</div>
<div id="notice" class="abs"></div>
<div id="noise" class="abs"></div>
<div id="orders"></div>
<div id="alerts"></div>
${opts.cameraButton ? '<button id="cam" class="abs" style="left:1100px;top:80px;width:80px;height:30px;z-index:10;opacity:0.99">Center</button>' : ""}
${opts.helpButton ? '<button id="help" class="abs">Help</button><div id="help_panel" class="abs">Help text</div>' : ""}
${opts.guide ? `<div id="guide" class="abs" style="background:${opts.guide === "opaque" ? "rgb(20,30,40)" : "rgba(20,30,40,0.8)"}"><div id="guide_copy" class="abs"></div><button id="guide_close" class="abs">Close</button></div>` : ""}
${opts.startOverlay ? '<div id="overlay" class="abs"><div id="start_text" class="abs" style="left:546px;top:346px;width:188px;height:29px;font-size:26px">Click to play</div></div>' : ""}
<script>
const OPTS = ${JSON.stringify(opts)};
const OWN = [[1, 360, 300], [2, 640, 430], [3, 900, 300]].slice(0, OPTS.units);
const ENEMY = [9, 1000, 520];
const NODE = [101, 560, 520];
let selection = [];
let notices = [];
let markers = 0;
let stateOff = false;
let camX = 0;
const $ = (id) => document.getElementById(id);
const units = [...OWN.map(([id, x, y]) => ({ id, x, y, owner: 0 })), { id: ENEMY[0], x: ENEMY[1], y: ENEMY[2], owner: 1 }];
function notice(text) { notices = [{ message: text, text }]; $("notice").style.display = "block"; $("notice").textContent = text; }
function clearSelection() { selection = []; $("ring").style.display = "none"; $("selection_panel").style.display = "none"; $("state").style.display = "none"; document.querySelectorAll(".order").forEach((el) => { el.style.display = "none"; }); }
function showOrders() { if (!OPTS.orders) return; document.querySelectorAll(".order").forEach((el) => { el.style.display = "block"; }); }
for (const unit of units) {
    const el = document.createElement("div");
    el.className = "abs unit" + (unit.owner ? " enemy" : "");
    el.style.left = (unit.x - 30) + "px"; el.style.top = (unit.y - 30) + "px";
    el.addEventListener("mousedown", (event) => {
        if (event.button !== 0) return;
        if (unit.owner !== 0) {
            selection = [unit.id];
            if (OPTS.enemyShows) { const panel = $("selection_panel"); panel.style.display = "block"; panel.textContent = "Enemy: Other player"; }
            return;
        }
        if (!OPTS.selectsNothing) selection = [unit.id];
        if (OPTS.selectShows) {
            const ring = $("ring");
            ring.style.display = "block"; ring.style.left = (unit.x - 42) + "px"; ring.style.top = (unit.y - 42) + "px";
            if (!OPTS.noPanel) { const panel = $("selection_panel"); panel.style.display = "block"; panel.textContent = "Selected: own" + unit.id; }
            if (selection.length) showOrders();
        }
    });
    $("units").appendChild(el);
}
$("node").addEventListener("mousedown", (event) => {
    if (event.button !== 0 || !OPTS.nodeShows) return;
    const panel = $("node_panel"); panel.style.display = "block"; panel.textContent = "Ore node";
});
$("map").addEventListener("mousedown", (event) => {
    if (event.button === 0 && !OPTS.groundKeeps) clearSelection();
    if (event.button === 2 && selection.length && !OPTS.rightDead) {
        const mark = $("mark"); mark.style.display = "block"; mark.style.left = (event.clientX - 15) + "px"; mark.style.top = (event.clientY - 15) + "px";
        markers += 1; notice("Moving.");
    }
});
document.addEventListener("contextmenu", (event) => event.preventDefault());
const ORDERS = [["Disable [D]", "fixture:order_disable"], ["Hold [H]", "fixture:order_hold"], ["Remove [X]", "fixture:order_remove"]];
ORDERS.forEach(([text, id], k) => {
    const el = document.createElement("button");
    el.className = "abs order"; el.id = "order" + k; el.style.top = (255 + 32 * k) + "px"; el.textContent = text;
    el.addEventListener("mousedown", () => {
        if (id === "fixture:order_disable") { if (!OPTS.ordersDead) { stateOff = !stateOff; $("state").style.display = "block"; $("state").textContent = stateOff ? "off" : "on"; } }
        else notice(id === "fixture:order_hold" ? "Holding position." : "Removed.");
    });
    $("orders").appendChild(el);
});
const alertTexts = [];
function addAlerts(count) {
    for (let k = 0; k < count; k += 1) {
        const text = "Alert " + alertTexts.length + ": something is waiting";
        const el = document.createElement("div");
        el.className = "abs alert"; el.style.top = (38 + 31 * alertTexts.length) + "px"; el.textContent = text;
        alertTexts.push(text);
        $("alerts").appendChild(el);
    }
}
if (OPTS.alertsLateMs > 0) setTimeout(() => addAlerts(OPTS.alerts), OPTS.alertsLateMs); else addAlerts(OPTS.alerts);
if (OPTS.noise) document.addEventListener("mousedown", () => setTimeout(() => { $("noise").style.display = "block"; if (alertTexts.length < 6) addAlerts(1); }, 100), true);
const copy = $("guide_copy");
if (copy) copy.textContent = Array.from({ length: OPTS.guideWords }, (_, k) => "word" + k).join(" ");
const overlay = $("overlay");
if (overlay) overlay.addEventListener("mousedown", () => overlay.remove());
const guide = $("guide");
if (guide) $("guide_close").addEventListener("mousedown", () => guide.remove());
const cam = $("cam");
if (cam) cam.addEventListener("mousedown", () => { camX += 40; });
const help = $("help");
let hovered = false;
if (help) { help.addEventListener("mouseenter", () => { hovered = OPTS.hover; }); help.addEventListener("mouseleave", () => { hovered = false; }); }
if (help && !OPTS.helpDead) help.addEventListener("mousedown", () => { const panel = $("help_panel"); panel.style.display = panel.style.display === "block" ? "none" : "block"; });
const rect = (el) => { const r = el.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; };
const shown = (el) => el && document.body.contains(el) && el.style.display !== "none" && getComputedStyle(el).display !== "none";
function widgets() {
    const list = [];
    // As the real client does, a widget whose place is outside the page is not visible in the record.
    const onPage = (r) => r[0] + r[2] / 2 >= 0 && r[1] + r[3] / 2 >= 0 && r[0] + r[2] / 2 < innerWidth && r[1] + r[3] / 2 < innerHeight;
    const w = (id, kind, el, extra = {}) => { const r = rect(el); list.push({ id, kind, rect: r, visible: onPage(r), enabled: true, capture_root: true, text: null, ...extra }); };
    const ov = $("overlay");
    if (ov && document.body.contains(ov)) list.push({ id: "focus_overlay", kind: "focus_overlay", rect: [0, 0, 1280, 720], visible: true, enabled: true, capture_root: true, focus_policy_block: true,
        text_nodes: [{ rect: rect($("start_text")), text: "Click to play", visible: true }] });
    const g = $("guide");
    if (g && document.body.contains(g)) { w("guide", "widget", g); w("guide_copy", "widget", copy, { text: copy.textContent }); w("guide_close", "button", $("guide_close"), { text: "Close" }); }
    document.querySelectorAll(".alert").forEach((el, k) => w("alert_rows/alert_line[" + k + "]", "button", el, { text: el.textContent }));
    if (cam) w("camera_button", "button", cam, { text: "Center", widget_action: { kind: "center_camera", x: 1, y: 1 } });
    if (help) { w("help", "button", help, { text: "Help" }); const p = $("help_panel"); if (p.style.display === "block") w("help_panel", "widget", p, { text: "Help text" }); }
    if (hovered) { list.push({ id: "help::tooltip", kind: "widget", rect: [1100, 52, 90, 20], visible: true, enabled: true, text: "Help tip" }); list.push({ id: "help_glow", kind: "widget", rect: [1096, 16, 88, 38], visible: true, enabled: true, text: null }); }
    if (shown($("selection_panel"))) w("selection_panel", "widget", $("selection_panel"), { text: $("selection_panel").textContent });
    if (shown($("node_panel"))) w("node_panel", "widget", $("node_panel"), { text: $("node_panel").textContent });
    if (shown($("state"))) w("state_label", "widget", $("state"), { text: $("state").textContent });
    if (shown($("notice"))) w("last_notice", "widget", $("notice"), { text: $("notice").textContent });
    document.querySelectorAll(".order").forEach((el, k) => { if (shown(el)) w("command_rows/command[" + k + "]", "button", el, { text: el.textContent, widget_action: { kind: "order", id: ORDERS[k][1] } }); });
    return list;
}
const debug = () => JSON.stringify({ phase: "Joined { slot: 0 }", slot: 0, selection, alerts: alertTexts.map((text) => ({ text })), notices, markers_shown: Array.from({ length: markers }, () => ({})),
    audio: { context_state: "running", sink_observations: [] },
    replicated_world: { units: units.map((u) => ({ unit_id: u.id, owner_slot: u.owner, content_id: "fixture:" + (u.owner ? "enemy" : "own") })) } });
window.turts_debug = debug;
window.turts_debug_request = () => Promise.resolve(debug());
window.turts_scene = () => JSON.stringify({ units: units.map((u) => ({ unit_id: u.id, translation: [u.x, 0, u.y], ground_screen_center: [u.x + camX, u.y], pick_radius: 30 })),
    nodes: [{ node_id: NODE[0], translation: [NODE[1], 0, NODE[2]], ground_screen_center: [NODE[1], NODE[2]], pick_radius: 30 }] });
window.turts_ui = () => JSON.stringify(widgets());
</script>`;
}

module.exports = { fixtureGame };
