# Fields I invented to write `frontier` (design rev 3 defines none of them)
Generator: gen.py (flags select each feature). Refusals of today's packer: stageB.txt.
Each row: name | type | default | validation | error text | source of the guess | conflict
- exports.combat | Vec<path> | [] | file exists | `manifest.ron: unknown field combat` | GD:268 "a combat export" | design names no export group
- combat file: damage_types, armour_classes, table[(damage, armour, ratio)], min, flat | ids, RefundRatio-like | none | missing cell? | none | GD:68 percent vs design:149 ratio,min,flat | home of min/flat undefined
- UnitDef.armour | Option<ContentRef> | None = ? (no class: ratio 1/1?) | ref exists | none | GD:266 | design 6.3 says "armour class on the unit", no field
- WeaponDef.damage_type | Option<ContentRef> | None = ? | ref exists | none | GD:267 |
- acquire (range, leash) | on WeaponDef (design:150) or UnitDef (GD:277) | None = 0.2.3 behaviour? | range >= weapon range? | none | conflict
- ResourceKind::Capacity(scope: Slot) | enum variant | - | - | - | design:142 "scope: Slot"; GD:32 `Network` | name conflict
- UnitDef.power / supplies / demands | (resource, amount) lists? | None | resource is Capacity | none | GD:270 `UnitDef.power`; the field name `power` equals frontier resource id `power` (no-content scan) |
- shortage effect | per capability (design:143) or per unit (GD:270) | none | - | - | conflict
- ProducerDef.queues [(id, recipes, capacity)] | - | one implicit queue? | ids unique | - | GD:272 `lanes`; design:250 "named queues" | queue ids "units"/"structures" collide with engine word `units` (scan probe)
- constructor / Construct order / Place behaviour | ? | - | - | - | design:71 `Construct`; GD:273 `Place`; UX:125 `Place` | conflict; job home contradiction design:132 vs :250
- UnitDef.placement (extends_area, gap, node_clearance) | f32s | ? | gap >= largest mobile diameter + 0.2 ? | none | GD:275; design:217 |
- UnitDef.repairer (rate, reach, cost per HP, classes, auto_range) | cost per HP as integer ratio? | - | work capability or not? | - | GD:276 |
- PaymentPolicy::ChargeOverTime | variant | - | - | - | design:157; GD:271 `ConsumeOverTime` | name conflict; multi-resource atomicity undefined
- Behaviour::SetRally, Repair; SmartTarget::OwnDamaged; needs Repairer | variants | - | - | - | UX:126 | Capability enum has no Repairer/Constructor
- SlotRelation::Ally, Enemy (design:173) vs Enemy, Ally, Own (GD:278)
- victory / match: LastTeamStanding(key_units) home: map? manifest? new export? | GD:281 `match` export | today: map refuses `match`
- disconnect defeat N=120 s field | Option<u32> | default must be None (D28 Decided) | - | - | design:44 "mod data"
- start policy AllReady(countdown) | UX:101 | home undefined
- lobby palette (slot_colours of hud.ron? lobby def?) | UX:101 gives hud slot_colours | design:385 "mod palette"
- HUD: Supply(id, Made|Used), SupplyShort(id), alert LowPower, NoCredits, cursor keys place/repair/rally, reason keys for 4 placement causes | UX:116-123 | reason and cursor keys are not validated today (exit 0 on typos)
- assets.models [(id, path)], VisualDef.model (asset, team_material, turret_node, muzzle_node) | RE:16-17 | caps undecided (RE:24) | design 8.1 gives no field
- effect ids per weapon/unit (flash, tracer, burst, sound) | RE part 4 | WeaponDef has no id; Fired{weapon} names it
- sounds cap per effect (AUD2 "mod cap") | home undefined
