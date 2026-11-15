// OR9. Build a private mod fixture and change its supply with ordinary mouse orders.
// PREPARED must contain the operating-resources backend and matching browser client.
const fs = require('node:fs');
const net = require('node:net');
const {spawn, spawnSync} = require('node:child_process');
const {chromium} = require('playwright-core');
const [prepared, out, mode] = process.argv.slice(2);
if (!out || (mode && !['--fixture-only', '--independent'].includes(mode))) throw Error('PREPARED OUTPUT [--fixture-only|--independent]');
fs.mkdirSync(out, {recursive: true});
const fixture = out + '/operating';
fs.cpSync(prepared + '/pack/skirmish', fixture, {recursive: true});
function qualify(directory) {
 for (const entry of fs.readdirSync(directory, {withFileTypes: true})) {
  const file = directory + '/' + entry.name;
  if (entry.isDirectory()) qualify(file);
  else if (entry.name.endsWith('.ron')) fs.writeFileSync(file, fs.readFileSync(file, 'utf8').replaceAll('skirmish:', 'operating:'));
 }
}
qualify(fixture);
const write = (path, text) => fs.writeFileSync(fixture + '/' + path, text);
const basic = (id, name, fields = '', radius = 1.5, health = 200) => `(id:"${id}",name:"${name}",footprint:Circle(radius:${radius}),movement:None,inventory:None,gatherer:None,drop_off:None,producer:None,health:Some((max:${health})),removal:Some((by_command:true,contents:Destroy))${fields})`;
write('resources/power.ron', '(id:"power",name:"Power",unit_name:"power",kind:Supply(scope:Slot,counts:Completed))');
write('resources/population.ron', '(id:"population",name:"Cots",unit_name:"cots",kind:Supply(scope:Slot,counts:Completed))');
write('units/provider.ron', basic('provider', 'Generator', ',provides:[(resource:"operating:power",amount:10),(resource:"operating:population",amount:6)],placement:Some((gap:1.5,node_clearance:0.0,in_build_area:true,site_health:(numerator:1,denominator:4)))'));
write('units/foreign_provider.ron', basic('foreign_provider', 'Foreign generator', ',provides:[(resource:"operating:power",amount:77),(resource:"operating:population",amount:88)]'));
write('units/workshop.ron', `(id:"workshop",name:"Workshop",footprint:Circle(radius:1.5),movement:None,inventory:None,gatherer:None,drop_off:None,
 producer:Some((recipes:["operating:train_trainee"],queue_capacity:2,spawn_offsets:[(x:3.5,y:0.0)],when_short:[(resource:"operating:power",effect:Rate((numerator:1,denominator:2))),(resource:"operating:population",effect:Rate((numerator:3,denominator:4)))])),
 constructor:Some((buildings:["operating:build_provider"],concurrent_sites:2,queue_capacity:2,when_short:[(resource:"operating:power",effect:Rate((numerator:1,denominator:2))),(resource:"operating:population",effect:Rate((numerator:1,denominator:3)))])),
 build_area:Some(16.0),consumes:[(resource:"operating:power",amount:4),(resource:"operating:population",amount:2)],health:Some((max:500)),removal:Some((by_command:true,contents:Destroy)))`);
write('units/test_tower.ron', basic('test_tower', 'Supply tower', ',consumes:[(resource:"operating:power",amount:2)],acquire:Some((range:9.0,leash:0.0)),weapon:Some((range:9.0,range_mode:TargetSurface,cooldown_ms:500,damage:5,ammo:None,targets:(slots:Enemy),when_short:[(resource:"operating:power",effect:Disable)]))', 1.0, 400));
write('units/mint.ron', basic('mint', 'Mint', ',income:Some((resource:"operating:credits",amount:3,period_ms:1050))', 2.0));
write('units/target.ron', basic('target', 'Target', '', 2.0, 1000000));
write('units/trainee.ron', '(id:"trainee",name:"Trainee",footprint:Circle(radius:0.5),movement:Some((speed:3.5)),inventory:None,gatherer:None,drop_off:None,producer:None,health:Some((max:60)),removal:Some((by_command:true,contents:Destroy)))');
const refunds = '(queued:(numerator:1,denominator:1),running:(numerator:1,denominator:2),output_blocked:(numerator:1,denominator:2))';
write('production/build_provider.ron', `(id:"build_provider",name:"Build Generator",output_unit:"operating:provider",duration_ms:6000,costs:[(resource:"operating:credits",amount:20,source:SlotBalance)],payment:ChargeOnQueue,cancel_refunds:${refunds})`);
write('production/train_trainee.ron', `(id:"train_trainee",name:"Train Trainee",output_unit:"operating:trainee",duration_ms:12000,costs:[(resource:"operating:credits",amount:10,source:SlotBalance)],payment:ChargeOnQueue,cancel_refunds:${refunds})`);
const visualKinds = {provider: 'barracks', foreign_provider: 'barracks', workshop: 'barracks', test_tower: 'tower', mint: 'base', target: 'base', trainee: 'worker'};
for (const [kind, original] of Object.entries(visualKinds)) write('visuals/' + kind + '.ron', fs.readFileSync(fixture + '/visuals/' + original + '.ron', 'utf8').replace(`Unit("operating:${original}")`, `Unit("operating:${kind}")`));
const all = Object.keys(visualKinds).map(id => `"operating:${id}"`).join(',');
function order(id, name, behaviour, target, units) {
 write('orders/' + id + '.ron', `(id:"${id}",name:"${name}",behaviour:${behaviour},target:${target},fire:None,engage:None,icon:"icon_order_produce",hotkey:"fixture_order",cursor:None,card:None,units:[${units}],description:"${name}.")`);
}
order('fixture_remove', 'Remove', 'Remove', 'None', all);
order('fixture_disable', 'Disable', 'SetEnabled(false)', 'None', '"operating:test_tower"');
order('fixture_enable', 'Enable', 'SetEnabled(true)', 'None', '"operating:test_tower"');
order('fixture_produce', 'Train', 'Produce', 'Content', '"operating:workshop"');
order('fixture_construct', 'Build', 'Construct', 'ContentAtPoint', '"operating:workshop"');
write('input/bindings.ron', '(actions:[(name:"fixture_order",default:[])],engine_overrides:[])');
write('orders/rules.ron', '(queue_depth:16,smart:[(target:Content,needs:Producer,order:"operating:fixture_produce")])');
let manifest = fs.readFileSync(fixture + '/manifest.ron', 'utf8');
manifest = manifest.replace('id: "skirmish"', 'id: "operating"')
 .replace(/engine_api:\s*"[^"]+"/, 'engine_api: "^0.2.13"')
 .replace('units: [', 'units: [' + Object.keys(visualKinds).map(id => `"units/${id}.ron"`).join(',') + ',')
 .replace('visuals: [', 'visuals: [' + Object.keys(visualKinds).map(id => `"visuals/${id}.ron"`).join(',') + ',')
 .replace('resources: [', 'resources: ["resources/power.ron","resources/population.ron",')
 .replace('production: [', 'production: ["production/build_provider.ron","production/train_trainee.ron",')
 .replace(/orders:\s*\[[^\]]*\]/, 'orders: ["orders/fixture_remove.ron","orders/fixture_disable.ron","orders/fixture_enable.ron","orders/fixture_produce.ron","orders/fixture_construct.ron","orders/rules.ron"]');
write('manifest.ron', manifest);
const place = (kind, owner, x, y) => `(unit:"operating:${kind}",owner_slot:${owner},pos:(x:${x}.0,y:${y}.0),contents:[])`;
write('maps/field.ron', `(id:"field",size:(x:64.0,y:48.0),placements:[${[
 place('provider', 0, 12, 16), place('workshop', 0, 16, 24), place('test_tower', 0, 28, 24),
 place('mint', 0, 14, 33), place('target', 1, 36, 24), place('foreign_provider', 1, 54, 12),
].join(',')}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"operating:credits",amount:500)]),(slot:1,balances:[(balance:"operating:credits",amount:0)])])`);
write('match.ron', '(start:Immediate,victory:None,open_sites_per_slot:Some(4))');
const text = (id, y, value, visible = '[]') => `(id:"${id}",at:Some((anchor:TopLeft,offset:(16.0,${y}.0),pivot:None)),size:(w:Px(294.0),h:Px(26.0)),visible:${visible},widget:Text(text:${value},font:None,size:14,colour:"#ffffff",align:Left,wrap:true))`;
const button = (id, y, name, action, visible = '[PrimaryOwned]') => `(id:"${id}",at:Some((anchor:TopLeft,offset:(16.0,${y}.0),pivot:None)),size:(w:Px(294.0),h:Px(28.0)),visible:${visible},widget:Button(icon:None,label:Some(Literal("${name}")),action:Order("operating:${action}"),hotkey_label:false))`;
const rootWidgets = [
 '(id:"backdrop",at:Some((anchor:TopLeft,offset:(6.0,6.0),pivot:None)),size:(w:Px(310.0),h:Px(690.0)),layer:-1,widget:Box(fill:Some(Colour("#10251c")),border:None,children:[]))',
 text('power', 12, 'Format("Power: {0} / {1}",[Supply("operating:power",Provided),Supply("operating:power",Consumed)])'),
 text('population', 42, 'Format("Cots: {0} / {1}",[Supply("operating:population",Provided),Supply("operating:population",Consumed)])'),
 text('credits', 72, 'Format("Credits: {0}",[Balance("operating:credits")])'),
 text('power_short', 102, 'Literal("Power shortage")', '[SupplyShort("operating:power")]'),
 text('population_short', 128, 'Literal("Cots shortage")', '[SupplyShort("operating:population")]'),
 text('selection_name', 164, 'Bind(SelectionPrimary(Name))', '[SelectionNotEmpty]'),
 button('remove', 194, 'Remove', 'fixture_remove'),
 button('disable', 228, 'Disable', 'fixture_disable', '[PrimaryOwned,PrimaryHas(Weapon)]'),
 button('enable', 262, 'Enable', 'fixture_enable', '[PrimaryOwned,PrimaryHas(Weapon)]'),
 '(id:"build_menu",at:Some((anchor:TopLeft,offset:(16.0,304.0),pivot:None)),size:(w:Px(294.0),h:Px(100.0)),visible:[PrimaryOwned],widget:List(source:BuildMenu,direction:Vertical,gap:4,max_items:3,item:(id:"build_item",size:(w:Fill,h:Px(28.0)),widget:Button(icon:None,label:Some(Item(Name)),action:Item(Action),hotkey_label:false))))',
 '(id:"work",at:Some((anchor:TopLeft,offset:(16.0,418.0),pivot:None)),size:(w:Px(294.0),h:Px(56.0)),visible:[PrimaryOwned],widget:List(source:ProductionQueue,direction:Vertical,gap:4,max_items:1,item:(id:"work_row",size:(w:Fill,h:Px(26.0)),widget:Text(text:Format("Work: {0}",[Item(Progress)]),font:None,size:12,colour:"#ffffff",align:Left,wrap:false))))',
 '(id:"sites",at:Some((anchor:TopLeft,offset:(16.0,482.0),pivot:None)),size:(w:Px(294.0),h:Px(56.0)),widget:List(source:Sites,direction:Vertical,gap:4,max_items:1,item:(id:"site_row",size:(w:Fill,h:Px(26.0)),widget:Text(text:Format("Site: {0}",[Item(Progress)]),font:None,size:12,colour:"#ffffff",align:Left,wrap:false))))',
 '(id:"alerts",at:Some((anchor:TopLeft,offset:(16.0,570.0),pivot:None)),size:(w:Px(294.0),h:Px(100.0)),widget:List(source:Alerts,direction:Vertical,gap:4,max_items:3,item:(id:"alert_row",size:(w:Fill,h:Px(26.0)),widget:Text(text:Item(Name),font:None,size:12,colour:"#ffffff",align:Left,wrap:false))))',
];
write('interface/hud.ron', `(id:"operating_hud",root:[${rootWidgets.join(',')}],tooltip_style:(background:"#000000",colour:"#ffffff",font:None,size:12),cursors:{},camera:(pitch:0.7,yaw:0.0,field_of_view:60.0,minimum:10.0,maximum:100.0,start:OwnedCentre,height:36.0,margin:0.0,pan_speed:100.0,edge_band:false),light:None,slot_colours:["#0072B2","#E69F00"],world:(ground:"#243b2c",grid:"#354b3c",selection:"#ffffff",health:"#00ff00",range:"#ffffff",show_range:true,tooltip:(background:"#000000",colour:"#ffffff",font:None,size:12),health_bar:(width:32,height:4,background:"#000000",health:"#00ff00")),alerts:[
 (id:"power_short_event",trigger:SupplyShort("operating:power"),units:[],offscreen_only:false,sound:None,text:"Power supply is short",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:0.0),
 (id:"power_restored_event",trigger:SupplyRestored("operating:power"),units:[],offscreen_only:false,sound:None,text:"Power supply restored",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:0.0)
 ],reasons:{},units:{},selection_priority:[],guide:[])`);

const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const freePort = () => new Promise(resolve => {const server = net.createServer(); server.listen(0, '127.0.0.1', () => {const port = server.address().port; server.close(() => resolve(port));});});
function project(scene, position) {
 const matrix = scene.clip_from_world;
 const clip = [0, 1, 2, 3].map(row => [...position, 1].reduce((sum, value, column) => sum + matrix[column * 4 + row] * value, 0));
 const [x, y, width, height] = scene.viewport_logical_rect;
 return [x + (clip[0] / clip[3] + 1) * width / 2, y + (1 - clip[1] / clip[3]) * height / 2];
}
(async () => {
 if (mode === '--fixture-only') {console.log(JSON.stringify({fixture, status: 'WRITTEN_NOT_RUN'})); return;}
 let server, context, page, peerContext, read;
 const profiles = [];
 const result = {status: 'RUNNING', errors: [], fixture};
 const widget = (state, id) => state.ui.find(row => row.visible && row.id === id);
 const units = state => state.debug.replicated_world.units;
 const health = (state, id) => state.debug.replicated_world.healths.find(row => row.unit === id)?.current;
 const tick = state => state.debug.replicated_world.tick;
 const job = state => state.debug.replicated_world.economy.queues.find(row => row.producer === 2)?.jobs[0];
 const alerts = (state, id) => state.debug.alerts.filter(row => row.id === id);
 const launch = async () => {
  const profile = fs.mkdtempSync('/tmp/turts-operating-'); profiles.push(profile);
  return chromium.launchPersistentContext(profile, {executablePath: '/usr/bin/google-chrome', headless: false, viewport: {width: 1280, height: 720}, ignoreDefaultArgs: ['--enable-unsafe-swiftshader'], args: ['--enable-gpu', '--alsa-output-device=null', '--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--disable-vulkan-surface', '--no-first-run', '--no-default-browser-check']});
 };
 const readPage = target => target.evaluate(async () => {
  if (typeof turts_scene !== 'function' || !turts_scene()) return null;
  const debug = JSON.parse(await turts_debug_request());
  return {debug, selection: debug.selection, scene: JSON.parse(turts_scene()), ui: JSON.parse(turts_ui())};
 });
 try {
  const packed = spawnSync(prepared + '/bin/turts-pack', [fixture], {encoding: 'utf8'});
  fs.writeFileSync(out + '/pack.log', packed.stdout + packed.stderr);
  if (packed.status !== 0) throw Error('operating fixture pack failed');
  const http = await freePort(), ws = await freePort(), url = `http://127.0.0.1:${http}`;
  fs.writeFileSync(out + '/server.ron', `(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
  const fd = fs.openSync(out + '/server.log', 'w');
  server = spawn(prepared + '/bin/turts_server', [out + '/server.ron'], {stdio: ['ignore', fd, fd]}); fs.closeSync(fd);
  for (let i = 0; i < 300; i++) {try {if ((await fetch(url + '/advertisement.json')).ok) break;} catch {} if (server.exitCode !== null) throw Error('server stopped'); await sleep(50);}
  const navigate = target => target.goto(url + '/?server=' + encodeURIComponent(url) + '&ws=' + encodeURIComponent(`ws://127.0.0.1:${ws}`));
  context = await launch(); page = context.pages()[0]; page.on('pageerror', error => result.errors.push(String(error))); await navigate(page);
  read = () => readPage(page);
  const wait = async (test, label, ms = 30000, reader = read) => {
   const end = Date.now() + ms;
   while (Date.now() < end) {const state = await reader(); if (state && test(state)) return state; if (result.errors.length) throw Error(result.errors.join('\n')); await sleep(80);}
   throw Error(label + ' timeout');
  };
  const select = async id => {const state = await read(), unit = state.scene.units.find(row => row.unit_id === id); if (!unit) throw Error('unit missing ' + id); await page.mouse.click(...project(state.scene, unit.translation)); await wait(state => state.selection.length === 1 && state.selection[0] === id, 'select ' + id, 5000);};
  const clickButton = async label => {const state = await wait(state => state.ui.some(row => row.kind === 'button' && row.visible && row.enabled && row.text === label), label, 5000); const row = state.ui.find(row => row.kind === 'button' && row.visible && row.enabled && row.text === label); await page.mouse.click(row.rect[0] + row.rect[2] / 2, row.rect[1] + row.rect[3] / 2);};
  const totals = (state, power, population) => widget(state, 'power')?.text === power && widget(state, 'population')?.text === population;
  result.initial = await wait(state => state.debug.slot === 0 && state.scene.models?.length === 6 && state.scene.models.every(model => model.ready) && totals(state, 'Power: 10 / 6', 'Cots: 6 / 2'), 'initial private supply');
  if (alerts(result.initial, 'power_short_event').length || alerts(result.initial, 'power_restored_event').length) throw Error('initial snapshot emitted a supply transition');
  await page.mouse.click(640, 360); await page.screenshot({path: out + '/initial-totals.png'});
  result.income = [];
  const incomeStart = Date.now(); let previousBalance;
  while (Date.now() - incomeStart < 3500) {const state = await read(), balance = state.debug.replicated_world.economy.balances.find(row => row.resource === 'operating:credits').on_hand; if (balance !== previousBalance) {result.income.push({tick: tick(state), balance}); previousBalance = balance;} await sleep(100);}
  if (result.income.length < 3 || result.income.slice(1).some((row, index) => row.balance - result.income[index].balance !== 3)) throw Error('periodic income did not credit independent 3-credit payments');
  result.firing = await wait(state => health(state, 5) < 1000000, 'normally supplied tower fires');
  await select(2); await clickButton('Train Trainee');
  const fullStart = await wait(state => job(state)?.elapsed_ticks > 0, 'running producer');
  const fullEnd = await wait(state => tick(state) >= tick(fullStart) + 40, 'normal producer sample');
  result.full_speed = {ticks: tick(fullEnd) - tick(fullStart), work: job(fullEnd).elapsed_ticks - job(fullStart).elapsed_ticks};
  if (Math.abs(result.full_speed.work / result.full_speed.ticks - 1) > 0.05) throw Error('normal producer rate');
  await select(1); await clickButton('Remove');
  result.shortage = await wait(state => !units(state).some(unit => unit.unit_id === 1) && totals(state, 'Power: 0 / 6', 'Cots: 0 / 2') && widget(state, 'power_short') && widget(state, 'population_short') && alerts(state, 'power_short_event').length === 1, 'shortage transition');
  await page.screenshot({path: out + '/shortage.png'});
  let provider, siteProgress;
  if (mode !== '--independent') {
   await select(2); const beforeConstruct = (await read()).debug.command_results.length; await clickButton('Build Generator'); await sleep(150); await page.mouse.click(...project((await read()).scene, [20, 0, 16]));
   result.site = await wait(state => {
    const refused = state.debug.command_results.slice(beforeConstruct).find(row => row.refused?.length);
    if (refused) throw Error('replacement site refused: ' + JSON.stringify(refused));
    return units(state).some(unit => unit.content_id === 'operating:provider' && unit.unit_id !== 1) && state.ui.some(row => row.visible && row.id.includes('site_row') && row.text?.startsWith('Site: '));
   }, 'replacement site');
   provider = units(result.site).find(unit => unit.content_id === 'operating:provider').unit_id;
   siteProgress = state => Number(state.ui.find(row => row.visible && row.id.includes('site_row'))?.text.replace('Site: ', ''));
  } else await select(2);
  const shortStart = await read(), targetHealth = health(shortStart, 5);
  const shortEnd = await wait(state => tick(state) >= tick(shortStart) + 60, 'slowed work sample');
  result.slowed = {ticks: tick(shortEnd) - tick(shortStart), work: job(shortEnd).elapsed_ticks - job(shortStart).elapsed_ticks, site_progress: siteProgress ? siteProgress(shortEnd) - siteProgress(shortStart) : null};
  if (!Number.isFinite(result.slowed.work) || Math.abs(result.slowed.work / result.slowed.ticks - 0.5) > 0.04) throw Error('producer did not use the smaller one-half rate');
  if (siteProgress && (!Number.isFinite(result.slowed.site_progress) || Math.abs(result.slowed.site_progress * 120 / result.slowed.ticks - 1 / 3) > 0.04)) throw Error('constructor did not use its distinct one-third rate');
  if (health(shortEnd, 5) !== targetHealth) throw Error('weapon fired during shortage');
  if (alerts(shortEnd, 'power_short_event').length !== 1) throw Error('stable shortage repeated its alert');
  await page.screenshot({path: out + '/slowed-work.png'});
  await select(3); await clickButton('Disable');
  if (mode !== '--independent') {
  result.restored = await wait(state => health(state, provider) === 200 && totals(state, 'Power: 10 / 6', 'Cots: 6 / 2') && !widget(state, 'power_short') && !widget(state, 'population_short') && alerts(state, 'power_restored_event').length === 1, 'completed provider restores supply', 30000);
  const restoredStart = result.restored;
  result.manual_disabled = await wait(state => tick(state) >= tick(restoredStart) + 40, 'manual disable after recovery');
  if (health(result.manual_disabled, 5) !== health(restoredStart, 5)) throw Error('recovery enabled a manually disabled weapon');
  await page.screenshot({path: out + '/restored-totals.png'});
  await clickButton('Enable');
  result.firing_again = await wait(state => health(state, 5) < health(restoredStart, 5), 'manual enable restores firing');
  await wait(state => units(state).some(unit => unit.content_id === 'operating:trainee'), 'trainee completion');
  }
  peerContext = await launch(); const peer = peerContext.pages()[0]; peer.on('pageerror', error => result.errors.push(String(error))); await navigate(peer);
  result.peer = await wait(state => state.debug.slot === 1 && totals(state, 'Power: 77 / 0', 'Cots: 88 / 0'), 'other slot private totals', 30000, () => readPage(peer));
  await peer.screenshot({path: out + '/peer-private-totals.png'}); await peerContext.close(); peerContext = null; await page.bringToFront();
  const own = await read(); if (!totals(own, mode === '--independent' ? 'Power: 0 / 6' : 'Power: 10 / 6', mode === '--independent' ? 'Cots: 0 / 2' : 'Cots: 6 / 2')) throw Error('foreign supply replaced owner HUD');
  await wait(state => units(state).some(unit => unit.content_id === 'operating:trainee'), 'training queue clears before removal');
  const removeIds = provider ? [2, 3, provider] : [2, 3];
  await select(2); await page.keyboard.down('Shift');
  try {for (const id of removeIds.slice(1)) {const state = await read(); await page.mouse.click(...project(state.scene, state.scene.units.find(unit => unit.unit_id === id).translation));}} finally {await page.keyboard.up('Shift');}
  const selection = (await read()).selection;
  if (selection.length !== removeIds.length || !removeIds.every(id => selection.includes(id))) throw Error('remove group selection');
  await clickButton('Remove');
  result.zero = await wait(state => removeIds.every(id => !units(state).some(unit => unit.unit_id === id)) && totals(state, 'Power: 0 / 0', 'Cots: 0 / 0'), 'explicit zero totals');
  if (alerts(result.zero, 'power_short_event').length !== 1 || alerts(result.zero, 'power_restored_event').length !== 1) throw Error('unexpected supply transition count');
  await page.screenshot({path: out + '/zero-totals.png'});
  result.status = mode === '--independent' ? 'PARTIAL_PASS' : 'PASS';
  if (mode === '--independent') result.missing_checks = ['Constructor rate and provider completion', 'Restoration preserves manual weapon Disable', 'Weapon firing after restored supply'];
  console.log(JSON.stringify({status: result.status, full_speed: result.full_speed, slowed: result.slowed, provider}));
 } catch (error) {
  result.status = 'FAIL'; result.error = String(error.stack || error); if (read) try {result.failure = await read();} catch {} if (page) try {await page.screenshot({path: out + '/failure.png'});} catch {} console.error(error); process.exitCode = 1;
 } finally {
  if (peerContext) await peerContext.close(); if (context) await context.close();
  if (server && server.exitCode === null) {const ended = new Promise(resolve => server.once('exit', resolve)); server.kill('SIGTERM'); await ended;}
  for (const profile of profiles) fs.rmSync(profile, {recursive: true, force: true});
  result.profiles_removed = profiles.every(profile => !fs.existsSync(profile));
  fs.writeFileSync(out + '/result.json', JSON.stringify(result, null, 2));
 }
})();
