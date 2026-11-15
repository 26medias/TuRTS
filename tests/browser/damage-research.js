// DM9: real owner research updates existing and newly trained Warrior armour.
// The private package adds a research fixture. It preserves shipped combat stats.
const fs = require('node:fs');
const net = require('node:net');
const {spawn, spawnSync} = require('node:child_process');
const {chromium} = require('playwright-core');
const [prepared, out] = process.argv.slice(2);
if (!out) throw Error('PREPARED OUTPUT');
fs.mkdirSync(out, {recursive: true});
const fixture = out + '/fixture';
fs.cpSync(prepared + '/pack/skirmish', fixture, {recursive: true});
fs.mkdirSync(fixture + '/technologies', {recursive: true});
fs.writeFileSync(fixture + '/units/armour_lab.ron', `(
 id: "armour_lab", name: "Armour lab", footprint: Circle(radius: 1.5),
 movement: None, inventory: None, gatherer: None, drop_off: None, producer: None,
 research: Some((technologies: ["skirmish:armour_upgrade"], queue_capacity: 1)),
 health: Some((max: 200)), removal: Some((by_command: true, contents: Destroy)),
)`);
fs.writeFileSync(fixture + '/technologies/armour_upgrade.ron', `(
 id: "armour_upgrade", name: "Research Armour +1", prerequisites: [], duration_ms: 2000,
 costs: [(resource: "skirmish:credits", amount: 50, source: SlotBalance)],
 payment: ChargeOnQueue,
 cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 2), output_blocked: (numerator: 1, denominator: 2)),
 effects: [ArmourModifier(unit: "skirmish:warrior", stat: Flat, add: 1)],
)`);
fs.writeFileSync(fixture + '/visuals/armour_lab.ron',
 fs.readFileSync(fixture + '/visuals/barracks.ron', 'utf8').replace('Unit("skirmish:barracks")', 'Unit("skirmish:armour_lab")'));
fs.writeFileSync(fixture + '/orders/research.ron', `(id: "order_research", name: "Research an upgrade", behaviour: Produce, target: Content, fire: None, engage: None, icon: "icon_order_produce", hotkey: "key_produce", cursor: None, card: None, units: ["skirmish:armour_lab"], description: "Research an armour upgrade.")`);
let manifest = fs.readFileSync(fixture + '/manifest.ron', 'utf8');
manifest = manifest.replace('units: [', 'units: ["units/armour_lab.ron", ')
 .replace('visuals: [', 'visuals: ["visuals/armour_lab.ron", ')
 .replace('orders: [', 'technologies: ["technologies/armour_upgrade.ron"],\n  orders: ["orders/research.ron", ');
fs.writeFileSync(fixture + '/manifest.ron', manifest);
let rules = fs.readFileSync(fixture + '/orders/rules.ron', 'utf8');
rules = rules.replace('smart: [', 'smart: [(target: Content, needs: Research, order: "skirmish:order_research"),');
fs.writeFileSync(fixture + '/orders/rules.ron', rules);
let hud = fs.readFileSync(fixture + '/interface/hud.ron', 'utf8');
hud = hud.replace('    units: {', '    units: {\n        "skirmish:armour_lab": (icon: Some("portrait_barracks"), portrait: Some("portrait_barracks"), responses: {}),');
fs.writeFileSync(fixture + '/interface/hud.ron', hud);
const place = (kind, owner, x, y) => `(unit:"skirmish:${kind}",owner_slot:${owner},pos:(x:${x}.0,y:${y}.0),contents:[])`;
fs.writeFileSync(fixture + '/maps/field.ron', `(id:"field",size:(x:64.0,y:48.0),placements:[${[
 place('base', 0, 10, 24), place('barracks', 0, 16, 24), place('warrior', 0, 18, 29),
 place('armour_lab', 0, 16, 16), place('base', 1, 54, 24), place('warrior', 1, 44, 24),
].join(',')}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"skirmish:credits",amount:500)])])`);
fs.writeFileSync(fixture + '/match.ron', '(start:Immediate,victory:None,open_sites_per_slot:Some(4))');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const freePort = () => new Promise(resolve => {
 const server = net.createServer();
 server.listen(0, '127.0.0.1', () => {const port = server.address().port; server.close(() => resolve(port));});
});
function project(scene, position) {
 const matrix = scene.clip_from_world;
 const clip = [0, 1, 2, 3].map(row => [...position, 1].reduce((sum, value, column) => sum + matrix[column * 4 + row] * value, 0));
 const [x, y, width, height] = scene.viewport_logical_rect;
 return [x + (clip[0] / clip[3] + 1) * width / 2, y + (1 - clip[1] / clip[3]) * height / 2];
}
(async () => {
 let server, context, page, read;
 const profile = fs.mkdtempSync('/tmp/turts-damage-research-');
 const result = {status: 'RUNNING', errors: [], profile, fixture_changes: 'Research lab and +1 Flat Warrior technology. Shipped units, armour, damage, costs, and training times are unchanged.'};
 try {
  const packed = spawnSync(prepared + '/bin/turts-pack', [fixture], {encoding: 'utf8'});
  fs.writeFileSync(out + '/pack.log', packed.stdout + packed.stderr);
  if (packed.status !== 0) throw Error('fixture pack failed');
  const http = await freePort(), ws = await freePort(), url = `http://127.0.0.1:${http}`;
  fs.writeFileSync(out + '/server.ron', `(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
  const fd = fs.openSync(out + '/server.log', 'w');
  server = spawn(prepared + '/bin/turts_server', [out + '/server.ron'], {stdio: ['ignore', fd, fd], env: {...process.env, TURTS_COMPUTER_SLOT: ''}});
  fs.closeSync(fd);
  for (let i = 0; i < 300; i++) {
   try {if ((await fetch(url + '/advertisement.json')).ok) break;} catch {}
   if (server.exitCode !== null) throw Error('server stopped');
   await sleep(50);
  }
  context = await chromium.launchPersistentContext(profile, {
   executablePath: '/usr/bin/google-chrome', headless: false, viewport: {width: 1280, height: 720},
   ignoreDefaultArgs: ['--enable-unsafe-swiftshader'],
   args: ['--enable-gpu', '--alsa-output-device=null', '--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--disable-vulkan-surface', '--no-first-run', '--no-default-browser-check'],
  });
  page = context.pages()[0];
  page.on('pageerror', error => result.errors.push(String(error)));
  await page.goto(url + '/?server=' + encodeURIComponent(url) + '&ws=' + encodeURIComponent(`ws://127.0.0.1:${ws}`));
  read = () => page.evaluate(async () => {
   if (typeof turts_scene !== 'function' || !turts_scene()) return null;
   const debug = JSON.parse(await turts_debug_request());
   return {debug, selection: debug.selection, scene: JSON.parse(turts_scene()), ui: JSON.parse(turts_ui())};
  });
  const wait = async (test, label, ms = 60000) => {
   const end = Date.now() + ms;
   while (Date.now() < end) {
    const state = await read();
    if (state && test(state)) return state;
    if (result.errors.length) throw Error(result.errors.join('\n'));
    await sleep(80);
   }
   throw Error(label + ' timeout');
  };
  const widget = (state, id) => state.ui.find(row => row.id === id && row.visible);
  const clickUnit = async id => {
   const state = await read(), unit = state.scene.units.find(row => row.unit_id === id);
   if (!unit) throw Error('unit missing: ' + id);
   await page.mouse.click(...project(state.scene, unit.translation));
   await wait(state => state.selection.length === 1 && state.selection.includes(id), 'select unit ' + id, 5000);
  };
  const button = async label => {
   const state = await wait(state => state.ui.some(row => row.kind === 'button' && row.visible && row.enabled && row.text?.includes(label)), label, 5000);
   const row = state.ui.find(row => row.kind === 'button' && row.visible && row.enabled && row.text?.includes(label));
   await page.mouse.click(row.rect[0] + row.rect[2] / 2, row.rect[1] + row.rect[3] / 2);
  };
  await wait(state => state.scene.models?.length === 6 && state.scene.models.every(model => model.ready), 'models');
  await page.mouse.click(640, 360);
  await clickUnit(3);
  result.before = await wait(state => widget(state, 'selection_armour')?.text === 'Armour: 2', 'existing Warrior armour 2');
  await page.screenshot({path: out + '/before-armour-2.png'});
  await clickUnit(4);
  result.lab = await wait(state => widget(state, 'selection_name')?.text === 'Armour lab', 'lab name');
  if (widget(result.lab, 'selection_armour')) throw Error('unit without armour has a nonempty armour row');
  result.unarmoured_row_absent = true;
  await page.screenshot({path: out + '/unarmoured-lab.png'});
  const started = Date.now();
  await button('Research Armour +1');
  result.queued = await wait(state => state.ui.some(row => row.visible && row.id.startsWith('m2_job_line') && row.text?.includes('Research Armour +1')), 'research queue', 5000);
  await page.screenshot({path: out + '/research-queue.png'});
  await clickUnit(3);
  result.existing_after = await wait(state => widget(state, 'selection_armour')?.text === 'Armour: 3', 'existing Warrior armour 3', 10000);
  result.research_ms = Date.now() - started;
  if (result.research_ms < 1900) throw Error('research completed before its configured duration');
  const research = result.existing_after.debug.replicated_world.research;
  if (!research.some(row => row.slot === 0 && row.completed.includes('skirmish:armour_upgrade') && row.armour_order?.includes('skirmish:armour_upgrade'))) throw Error('owner research or armour completion order missing');
  result.owner_research = research;
  await page.screenshot({path: out + '/existing-after-armour-3.png'});
  await clickUnit(2);
  const trainingStarted = Date.now();
  await button('Train Warrior');
  const trained = await wait(state => state.debug.replicated_world.units.some(unit => unit.owner_slot === 0 && unit.content_id === 'skirmish:warrior' && unit.unit_id !== 3) && state.scene.models.every(model => model.ready), 'new Warrior', 15000);
  const newWarrior = trained.debug.replicated_world.units.find(unit => unit.owner_slot === 0 && unit.content_id === 'skirmish:warrior' && unit.unit_id !== 3).unit_id;
  result.training_ms = Date.now() - trainingStarted;
  if (result.training_ms < 5900) throw Error('new Warrior trained before its configured duration');
  result.new_warrior_id = newWarrior;
  await clickUnit(newWarrior);
  result.new_after = await wait(state => widget(state, 'selection_armour')?.text === 'Armour: 3', 'new Warrior armour 3', 5000);
  await page.screenshot({path: out + '/new-after-armour-3.png'});
  await clickUnit(6);
  result.enemy = await read();
  if (widget(result.enemy, 'selection_armour')) throw Error('enemy armour row leaked');
  if (result.enemy.debug.replicated_world.research.some(row => row.slot !== 0)) throw Error('enemy research leaked');
  result.enemy_armour_absent = true;
  await page.screenshot({path: out + '/enemy-armour-hidden.png'});
  result.status = 'PASS';
  console.log(JSON.stringify({status: result.status, research_ms: result.research_ms, training_ms: result.training_ms, new_warrior_id: newWarrior}));
 } catch (error) {
  result.status = 'FAIL'; result.error = String(error.stack || error);
  if (read) try {result.failure = await read();} catch {}
  if (page) try {await page.screenshot({path: out + '/failure.png'});} catch {}
  console.error(error); process.exitCode = 1;
 } finally {
  if (context) await context.close();
  if (server && server.exitCode === null) {const ended = new Promise(resolve => server.once('exit', resolve)); server.kill('SIGTERM'); await ended;}
  fs.rmSync(profile, {recursive: true, force: true});
  result.profile_removed = !fs.existsSync(profile);
  fs.writeFileSync(out + '/result.json', JSON.stringify(result, null, 2));
 }
})();
