import os, sys, shutil
# Generate the frontier package, stage A (today's schema) or with design extensions (stage B, by flags).
out = sys.argv[1]
flags = set(sys.argv[2:])
shutil.rmtree(out, ignore_errors=True)
def w(path, text):
    p = os.path.join(out, path); os.makedirs(os.path.dirname(p), exist_ok=True)
    open(p, "w").write(text)
P = "frontier"
units = {}
def wpn(dmg, rng, cd, dtype):
    extra = f", damage_type: Some(\"{P}:{dtype}\"), acquire: Some((range: {rng+2}.0, leash: 6.0))" if "dmg" in flags else ""
    return f"weapon: Some((range: {rng}.0, cooldown_ms: {cd}, damage: {dmg}, ammo: None, targets: (slots: Other){extra})), "
def extra_unit(armour, power=None, placement=None, repairer=None):
    s = ""
    if "dmg" in flags: s += f", armour: Some(\"{P}:{armour}\")"
    if "pwr" in flags and power: s += f", power: Some({power})"
    if "place" in flags and placement: s += f", placement: Some({placement})"
    if "rep" in flags and repairer: s += f", repairer: Some({repairer})"
    return s
mob = [("rifleman","Rifleman",0.4,2.4,120,(12,5,1000,"bullet")),
       ("launcher","Launcher",0.4,2.0,90,(60,7,2500,"rocket")),
       ("engineer","Engineer",0.4,2.2,70,None),
       ("tank","Tank",0.9,2.6,500,(40,6,2000,"cannon"))]
for uid,name,r,spd,hp,wp in mob:
    armour = "vehicle" if uid=="tank" else "infantry"
    rep = "(rate_per_second: 8, reach: 1.5, cost: (resource: \"frontier:credits\", amount: 1, per_health: 2), targets: [\"frontier:vehicle\", \"frontier:structure\"], auto_range: 6.0)" if uid=="engineer" else None
    w(f"units/{uid}.ron", f"(id: \"{uid}\", name: \"{name}\", footprint: Circle(radius: {r}), movement: Some((speed: {spd})), inventory: None, gatherer: None, drop_off: None, producer: None, health: Some((max: {hp})), " + (wpn(*wp) if wp else "") + f"removal: Some((by_command: false, contents: Destroy)){extra_unit(armour, repairer=rep)})\n")
w("units/ore_hauler.ron", f"(id: \"ore_hauler\", name: \"Ore Hauler\", footprint: Circle(radius: 0.9), movement: Some((speed: 2.0)), inventory: Some((capacity: 200, accepts: [\"{P}:ore\"])), gatherer: Some((items: [\"{P}:ore\"], gather_per_second: 10, unload_per_second: 40, interaction_range: 0.5)), drop_off: None, producer: None, health: Some((max: 600)), removal: Some((by_command: false, contents: Destroy)){extra_unit('vehicle')})\n")
cc_queues = "recipes: [\"frontier:train_rifleman\", \"frontier:train_launcher\", \"frontier:train_engineer\", \"frontier:train_ore_hauler\", \"frontier:train_tank\", \"frontier:build_power_station\", \"frontier:build_guard_turret\"], queue_capacity: 5"
if "lanes" in flags:
    cc_queues = "recipes: [], queue_capacity: 5, queues: [(id: \"units\", recipes: [\"frontier:train_rifleman\", \"frontier:train_launcher\", \"frontier:train_engineer\", \"frontier:train_ore_hauler\", \"frontier:train_tank\"], capacity: 5), (id: \"structures\", recipes: [\"frontier:build_power_station\", \"frontier:build_guard_turret\"], capacity: 1)]"
cc_health = "health: Some((max: 2000)), removal: Some((by_command: false, contents: Destroy)), " if "cc_health" in flags else ""
cc_con = ", constructor: Some((buildings: [\"frontier:power_station\", \"frontier:guard_turret\"]))" if "con" in flags else ""
w("units/command_center.ron", f"(id: \"command_center\", name: \"Command Center\", footprint: Circle(radius: 2.5), movement: None, inventory: None, gatherer: None, drop_off: Some((rules: [(item: \"{P}:ore\", effect: Convert(balance: \"{P}:credits\", per_item: 1))])), producer: Some(({cc_queues}, spawn_offsets: [(x: 0.0, y: 4.5), (x: 0.0, y: -4.5), (x: 4.5, y: 0.0)])), {cc_health}output_inventory: None{cc_con}{extra_unit('structure', power='(supplies: [], demands: [(resource: \"frontier:power\", amount: 40)])', placement='(extends_area: Some(10.0), gap: 2.0, node_clearance: 2.0)')})\n")
w("units/power_station.ron", f"(id: \"power_station\", name: \"Power Station\", footprint: Circle(radius: 1.5), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None, health: Some((max: 600)), removal: Some((by_command: false, contents: Destroy)){extra_unit('structure', power='(supplies: [(resource: \"frontier:power\", amount: 100)], demands: [])', placement='(extends_area: Some(10.0), gap: 2.0, node_clearance: 2.0)')})\n")
tw = wpn(40,8,1500,"cannon")
if "pwr" in flags: tw = tw.replace("targets: (slots: Other)", "targets: (slots: Other), shortage: Some(Disable)")
w("units/guard_turret.ron", f"(id: \"guard_turret\", name: \"Guard Turret\", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None, health: Some((max: 600)), {tw}removal: Some((by_command: false, contents: Destroy)){extra_unit('structure', power='(supplies: [], demands: [(resource: \"frontier:power\", amount: 30)])', placement='(extends_area: None, gap: 2.0, node_clearance: 2.0)')})\n")
uids = ["rifleman","launcher","engineer","ore_hauler","tank","command_center","power_station","guard_turret"]
pay = "ChargeOverTime" if "pay" in flags else "ChargeOnQueue"
prods = [("train_rifleman","rifleman",6000,100),("train_launcher","launcher",9000,250),("train_engineer","engineer",10000,300),("train_ore_hauler","ore_hauler",20000,800),("train_tank","tank",18000,700)]
if "build_prod" in flags:
    prods += [("build_power_station","power_station",12000,300),("build_guard_turret","guard_turret",20000,500)]
for pid,out_u,ms,cost in prods:
    w(f"production/{pid}.ron", f"(id: \"{pid}\", name: \"{pid.replace('_',' ').title()}\", output_unit: \"{P}:{out_u}\", duration_ms: {ms}, costs: [(resource: \"{P}:credits\", amount: {cost}, source: SlotBalance)], payment: {pay}, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))\n")
if "build_prod" not in flags:
    w("units/command_center.ron", open(os.path.join(out,"units/command_center.ron")).read().replace(', \\"frontier:build_power_station\\", \\"frontier:build_guard_turret\\"', ''))
    cc = open(os.path.join(out,"units/command_center.ron")).read().replace(', "frontier:build_power_station", "frontier:build_guard_turret"','')
    cc = cc.replace('(id: "structures", recipes: [\"frontier:build_power_station\", \"frontier:build_guard_turret\"], capacity: 1)', '(id: "structures", recipes: [], capacity: 1)')
    w("units/command_center.ron", cc)
w("resources/ore.ron", "(id: \"ore\", name: \"Ore\", unit_name: \"ore\", kind: Item)\n")
w("resources/credits.ron", "(id: \"credits\", name: \"Credits\", unit_name: \"credit\", kind: Balance)\n")
res = ["resources/ore.ron","resources/credits.ron"]
if "pwr" in flags:
    w("resources/power.ron", "(id: \"power\", name: \"Power\", unit_name: \"power\", kind: Capacity(scope: Slot))\n"); res.append("resources/power.ron")
w("nodes/ore_field.ron", f"(id: \"ore_field\", name: \"Ore Field\", item: \"{P}:ore\", radius: 1.0)\n")
vis = {"rifleman":"Capsule(radius: 0.3, height: 1.8)","launcher":"Capsule(radius: 0.3, height: 1.7)","engineer":"Capsule(radius: 0.3, height: 1.6)","tank":"Cuboid(x: 1.8, y: 1.0, z: 1.4)","ore_hauler":"Cuboid(x: 1.8, y: 1.2, z: 1.4)","command_center":"Cuboid(x: 4.0, y: 3.0, z: 4.0)","power_station":"Cuboid(x: 2.4, y: 2.0, z: 2.4)","guard_turret":"Cuboid(x: 1.6, y: 1.6, z: 1.6)"}
for u,s in vis.items():
    m = f", model: Some((asset: \"{P}_model_{u}\", team_material: \"team\", turret_node: None, muzzle_node: None))" if "model" in flags else ""
    w(f"visuals/{u}.ron", f"(target: Unit(\"{P}:{u}\"), shape: {s}{m})\n")
w("visuals/ore_field.ron", f"(target: Node(\"{P}:ore_field\"), shape: Cuboid(x: 2.0, y: 1.0, z: 2.0))\n")
orders = {
 "order_move": ("Move","PointOrUnit","Never","None", ["rifleman","launcher","engineer","ore_hauler","tank"],"key_move"),
 "order_stop": ("Stop","None","None","None", ["rifleman","launcher","engineer","ore_hauler","tank","guard_turret"],"key_stop"),
 "order_hold": ("Hold","None","InRange","None", ["rifleman","launcher","engineer","tank","guard_turret"],"key_hold"),
 "order_attack": ("Attack","PointOrUnit","None","StopAndFire", ["rifleman","launcher","tank","guard_turret"],"key_attack"),
 "order_gather": ("Gather","Node","None","None", ["ore_hauler"],"key_gather"),
 "order_produce": ("Produce","Content","None","None", ["command_center"],"key_produce"),
 "order_cancel": ("CancelProduction","Job","None","None", ["command_center"],"key_cancel"),
}
if "neworders" in flags:
    orders["order_construct"] = ("Construct","PointAndContent","None","None",["command_center"],"key_construct")
    orders["order_repair"] = ("Repair","Unit","None","None",["engineer"],"key_repair")
    orders["order_rally"] = ("SetRally","PointOrUnit","None","None",["command_center"],"key_rally")
for oid,(b,t,f,e,us,key) in orders.items():
    fire = "None" if f=="None" else f"Some({f})"; eng = "None" if e=="None" else f"Some({e})"
    w(f"orders/{oid}.ron", f"(id: \"{oid}\", name: \"{oid}\", behaviour: {b}, target: {t}, fire: {fire}, engage: {eng}, icon: \"{P}_icon\", hotkey: \"{key}\", cursor: None, card: None, units: [{', '.join(chr(34)+P+':'+u+chr(34) for u in us)}], description: \"{oid}\")\n")
smart = "(target: ForeignUnitWeaponTarget, needs: Weapon, order: \"frontier:order_attack\"), (target: ResourceNodeAccepted, needs: Gatherer, order: \"frontier:order_gather\"), (target: Ground, needs: Movement, order: \"frontier:order_move\"), (target: OwnUnit, needs: Movement, order: \"frontier:order_move\"), (target: Content, needs: Producer, order: \"frontier:order_produce\")"
if "neworders" in flags:
    smart += ", (target: OwnDamaged, needs: Repairer, order: \"frontier:order_repair\"), (target: Ground, needs: Producer, order: \"frontier:order_rally\")"
w("orders/rules.ron", f"(queue_depth: 16, smart: [{smart}])\n")
keys = sorted({v[5] for v in orders.values()})
codes = {"key_move":"KeyM","key_stop":"KeyS","key_hold":"KeyH","key_attack":"KeyA","key_gather":"KeyG","key_produce":"KeyQ","key_cancel":"KeyC","key_construct":"KeyB","key_repair":"KeyR","key_rally":"KeyY"}
w("input/bindings.ron", "(actions: [" + ", ".join(f"(name: \"{k}\", default: [Key(code: \"{codes[k]}\", mods: [])])" for k in keys) + "], engine_overrides: [])\n")
os.makedirs(os.path.join(out,"assets"), exist_ok=True)
shutil.copy("/home/julien/Projects/TuRTS-mp1/mods/base/assets/icon_order_move.png", os.path.join(out,"assets/icon.png"))
models = ""
if "model" in flags:
    models = ", models: [" + ", ".join(f"(id: \"{P}_model_{u}\", path: \"assets/models/{u}.glb\")" for u in vis) + "]"
w("assets/assets.ron", f"(images: [(id: \"{P}_icon\", path: \"assets/icon.png\", nine_slice: None)], fonts: [], sounds: []{models}, cursors: [])\n")
hud = open("/home/julien/Projects/TuRTS-mp1/mods/base/interface/hud.ron").read()
hud = hud.replace("base:", "frontier:").replace('icon_order_', 'x_')
# keep only build/job rows
lines = hud.split("\n")
hud = "\n".join(l for l in lines if "hud_order_" not in l)
if "hud" in flags:
    hud = hud.replace("    ],\n    tooltip_style", "        (id: \"power\", at: Some((anchor: Top, offset: (0.0, 4.0), pivot: None)), size: (w: Px(200.0), h: Px(24.0)), widget: Text(text: Format(\"Power {0} / {1}\", [Supply(\"frontier:power\", Made), Supply(\"frontier:power\", Used)]), font: None, size: 14, colour: \"#ffffff\", align: Centre, wrap: false)),\n        (id: \"low\", at: Some((anchor: Top, offset: (0.0, 30.0), pivot: None)), size: (w: Px(200.0), h: Px(24.0)), visible: [SupplyShort(\"frontier:power\")], widget: Text(text: Literal(\"Low power\"), font: None, size: 14, colour: \"#ff8800\", align: Centre, wrap: false)),\n    ],\n    tooltip_style")
    hud = hud.replace("alerts: [],", "alerts: [(id: \"low_power\", trigger: LowPower, sound: None, text: \"Low power\", minimap_ping: false, cooldown: 1.0, priority: 10, coalesce_radius: 5.0)],")
w("interface/hud.ron", hud)
pl = []
def add(u,s,x,y): pl.append(f"        (unit: \"{P}:{u}\", owner_slot: {s}, pos: (x: {x}, y: {y}), contents: []),")
add("command_center",0,16.0,36.0); add("ore_hauler",0,20.0,32.0); add("command_center",1,96.0,36.0); add("ore_hauler",1,92.0,40.0)
nodes = []
for (x,y) in [(14,22),(17.5,21),(21,22.5),(98,50),(94.5,51),(91,49.5),(14,58),(18,60),(98,14),(94,12)]:
    nodes.append(f"        (node: \"{P}:ore_field\", pos: (x: {float(x)}, y: {float(y)}), remaining: 1500),")
for (x,y) in [(54,14),(58,14),(56,11),(56,17),(58,58),(54,58),(56,61),(56,55)]:
    nodes.append(f"        (node: \"{P}:ore_field\", pos: (x: {float(x)}, y: {float(y)}), remaining: 2000),")
match = ""
if "match" in flags:
    match = ",\n    match: Some((victory: LastTeamStanding(key_units: [\"frontier:command_center\"]), disconnect_defeat_after_s: Some(120), start: AllReady(countdown_s: 3)))"
w("maps/twin_fields.ron", "(\n    id: \"twin_fields\",\n    size: (x: 112.0, y: 72.0),\n    placements: [\n" + "\n".join(pl) + "\n    ],\n    nodes: [\n" + "\n".join(nodes) + "\n    ],\n    slot_economy: [\n        (slot: 0, balances: [(balance: \"frontier:credits\", amount: 1200)]),\n        (slot: 1, balances: [(balance: \"frontier:credits\", amount: 1200)]),\n    ]" + match + ",\n)\n")
exp_extra = ""
if "combat" in flags:
    w("combat/damage.ron", "(damage_types: [\"bullet\", \"rocket\", \"cannon\"], armour_classes: [\"infantry\", \"vehicle\", \"structure\"], table: [(damage: \"frontier:bullet\", armour: \"frontier:infantry\", ratio: (numerator: 1, denominator: 1)), (damage: \"frontier:bullet\", armour: \"frontier:vehicle\", ratio: (numerator: 1, denominator: 5))], min: 1, flat: 0)\n")
    exp_extra = '\n        combat: ["combat/damage.ron"],'
api = "^0.2.4" if "api24" in flags else "^0.2.3"
w("manifest.ron", f"""(
    id: "{P}",
    version: "0.1.0",
    kind: Game,
    engine_api: "{api}",
    dependencies: {{}},
    exports: (
        units: [{', '.join(f'"units/{u}.ron"' for u in uids)}],
        maps: ["maps/twin_fields.ron"],
        visuals: [{', '.join(f'"visuals/{u}.ron"' for u in list(vis)+['ore_field'])}],
        resources: [{', '.join(chr(34)+r+chr(34) for r in res)}],
        nodes: ["nodes/ore_field.ron"],
        production: [{', '.join(f'"production/{p[0]}.ron"' for p in prods)}],
        orders: [{', '.join(f'"orders/{o}.ron"' for o in orders)}, "orders/rules.ron"],
        interface: ["interface/hud.ron"],
        input: ["input/bindings.ron"],
        assets: ["assets/assets.ron"],{exp_extra}
    ),
    context: Shared,
    capabilities: [],
    license: "LicenseRef-TBD",
)
""")
