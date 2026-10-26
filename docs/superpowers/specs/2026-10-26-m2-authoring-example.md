# M2 data package authoring example — API 0.2 design

Status: **Approved design, not implemented**, companion to [M2 economy design](2026-10-26-m2-economy-design.md).
This is a complete example for the approved M2 schema, not a package supported by today's M1
loader. No files below are scripts or executable plugins. Values are correctness fixtures,
not game balance or capacity budgets. All nine manifest fields retain their M1 names.

## Serialization contract

At the authoring boundary, `ContentRef = String`: write qualified references as quoted strings,
for example `"cedar:wood"`. Loaders validate and resolve each to the internal ContentId.
Do **not** use ContentId's current derived `{package, local}` struct serialization in RON.
The proposed schema calls these serialized fields ContentRef; ContentId names their internal
resolved form. This retains M1's quoted-reference spelling while extending the schema.

Package ids match `[a-z0-9_]+` (M1 PackageId grammar). Local ids retain the M1 rule: nonempty
and containing no colon. References have exactly one
colon and match `<package>:<local>`. Definition `id` fields are local strings. References must
name a definition of the correct kind in this package; dependencies remain unsupported.
Definition ids are unique across resource, unit, node and production kinds in one package.

RON records use `(field: value)`, lists `[...]`, maps `{...}`, and enum variants as written.
Optional capabilities require `None` or `Some((...))`; an omitted field is an error. Integer
amounts are smallest resource units, duration_ms is milliseconds, rates are units/second.
WorldPos/MapSize use `(x: f32, y: f32)` on the ground plane. Footprint is
`Circle(radius: f32)`. Shapes are `Capsule(radius: f32, height: f32)` or
`Cuboid(x: f32, y: f32, z: f32)`; Cuboid z is vertical height, x/y are ground extents.
Visual target variants are `Unit("package:id")` and `Node("package:id")`.

## Complete `cedar` package

Create exactly these ten files; every exported path is relative to the package directory:

```text
cedar/manifest.ron
cedar/resources/wood.ron
cedar/units/depot.ron
cedar/units/worker.ron
cedar/nodes/grove.ron
cedar/production/hire.ron
cedar/maps/clearing.ron
cedar/visuals/depot.ron
cedar/visuals/worker.ron
cedar/visuals/grove.ron
```

`manifest.ron` (nine fields, all six export lists required):
```ron
(
    id: "cedar", version: "0.2.0", kind: Game, engine_api: "^0.2",
    dependencies: {},
    exports: (
        units: ["units/depot.ron", "units/worker.ron"],
        maps: ["maps/clearing.ron"],
        visuals: ["visuals/depot.ron", "visuals/worker.ron", "visuals/grove.ron"],
        resources: ["resources/wood.ron"], nodes: ["nodes/grove.ron"],
        production: ["production/hire.ron"],
    ),
    context: Shared, capabilities: [], license: "LicenseRef-TBD",
)
```

`resources/wood.ron`:
```ron
(id: "wood", name: "Timber", unit_name: "log", kind: Item)
```

`units/depot.ron`:
```ron
(
    id: "depot", name: "Timber Yard", footprint: Circle(radius: 2.0),
    movement: None,
    inventory: Some((capacity: 2000, accepts: ["cedar:wood"])),
    gatherer: None,
    drop_off: Some((rules: [(item: "cedar:wood", effect: Store)])),
    producer: Some((recipes: ["cedar:hire"], queue_capacity: 5,
        spawn_offsets: [(x: 0.0, y: 4.0), (x: 0.0, y: -4.0)])),
)
```

`units/worker.ron`:
```ron
(
    id: "worker", name: "Carrier", footprint: Circle(radius: 0.5),
    movement: Some((speed: 3.0)),
    inventory: Some((capacity: 10, accepts: ["cedar:wood"])),
    gatherer: Some((items: ["cedar:wood"], gather_per_second: 5,
        unload_per_second: 10, interaction_range: 0.5)),
    drop_off: None, producer: None,
)
```

`nodes/grove.ron`:
```ron
(id: "grove", name: "Timber Stand", item: "cedar:wood", radius: 1.0)
```

`production/hire.ron`:
```ron
(
    id: "hire", name: "Hire Carrier", output_unit: "cedar:worker",
    duration_ms: 6000,
    costs: [(resource: "cedar:wood", amount: 50, source: ProducerInventory)],
    payment: ReserveThenCharge,
    cancel_refunds: (
        queued: (numerator: 1, denominator: 1),
        running: (numerator: 1, denominator: 1),
        output_blocked: (numerator: 1, denominator: 1),
    ),
)
```

`maps/clearing.ron` (mirrored x coordinates across a 64-unit map):
```ron
(
    id: "clearing", size: (x: 64.0, y: 64.0),
    placements: [
        (unit: "cedar:depot", owner_slot: 0, pos: (x: 8.0, y: 8.0),
            contents: [(item: "cedar:wood", amount: 100)]),
        (unit: "cedar:worker", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: []),
        (unit: "cedar:worker", owner_slot: 0, pos: (x: 12.0, y: 12.0), contents: []),
        (unit: "cedar:depot", owner_slot: 1, pos: (x: 56.0, y: 8.0),
            contents: [(item: "cedar:wood", amount: 100)]),
        (unit: "cedar:worker", owner_slot: 1, pos: (x: 52.0, y: 8.0), contents: []),
        (unit: "cedar:worker", owner_slot: 1, pos: (x: 52.0, y: 12.0), contents: []),
    ],
    nodes: [
        (node: "cedar:grove", pos: (x: 24.0, y: 8.0), remaining: 1000),
        (node: "cedar:grove", pos: (x: 40.0, y: 8.0), remaining: 1000),
    ],
    slot_economy: [],
)
```

`visuals/depot.ron`:
```ron
(target: Unit("cedar:depot"), shape: Cuboid(x: 3.0, y: 3.0, z: 2.0))
```
`visuals/worker.ron`:
```ron
(target: Unit("cedar:worker"), shape: Capsule(radius: 0.3, height: 1.6))
```
`visuals/grove.ron`:
```ron
(target: Node("cedar:grove"), shape: Cuboid(x: 1.4, y: 1.4, z: 0.7))
```

The map seeds each yard with 100 logs before anyone claims a slot. There are no Balance
resources or accounts. Claiming creates nothing. Order a Carrier to a stand: it gathers into
its own 10-item inventory and stores at its owner's nearest accepting yard. Queueing Hire
reserves 50 logs at that producer only; processing starts by charging them. Completion adds
one empty Carrier at the first free output offset. Full-refund cancellation after charging
can reject if that producer's inventory cannot hold the returned logs; it never drops them.

## Valid conversion/balance variant

To change this same package into a converting global-balance economy, apply **all** changes:

1. Add `resources/vouchers.ron` below and append its path to exports.resources.
2. In depot, replace `effect: Store` with
   `effect: Convert(balance: "cedar:vouchers", per_item: 2)`. Keep the inventory declaration
   if desired; conversion credits the balance directly and does not deposit logs there.
3. In hire, replace the cost with
   `costs: [(resource: "cedar:vouchers", amount: 100, source: SlotBalance)]`, and payment with
   `ChargeOnQueue`. Keep the explicit three 1/1 cancellation ratios.
4. In the map, change both depot contents to `[]`, and replace slot_economy with the record
   below. Keep Item wood, worker inventory, finite nodes and node visuals unchanged.

`resources/vouchers.ron`:
```ron
(id: "vouchers", name: "Yard Vouchers", unit_name: "voucher", kind: Balance)
```
Replacement map field:
```ron
slot_economy: [
    (slot: 0, balances: [(balance: "cedar:vouchers", amount: 200)]),
    (slot: 1, balances: [(balance: "cedar:vouchers", amount: 200)]),
],
```

Now delivery consumes cargo logs and credits two vouchers per log; Hire debits 100 vouchers
on queue, never logs or a remote inventory. Cancellation refunds the charged account. A valid
role swap must change the definition, endpoint, seed state and receiver together.

## Pack and run after M2 implementation

From the repository root, `cargo run -p turts_pack -- /path/to/cedar` runs the shared server
and client checks, then writes `/path/to/cedar.tar` and prints archive path/hash. A bad package
prints ModError with package/file/field and exits 2 without an archive. The server reads this
package through `turts_pack::load_package`; there is no default-package shortcut.

Set `game_package: "/path/to/cedar.tar"` in the existing server.ron, retaining http_listen,
ws_listen, public_url, tick_hz and optional static_dir from M1. Run
`cargo run -p turts_server -- /path/to/server.ron`. For a native diagnostic join use
`cargo run -p turts_client -- --headless --server http://127.0.0.1:7000 --ws ws://127.0.0.1:7001
--run-secs 4 --out /tmp/cedar-client.json`, matching your configured ports. Build the browser
with `trunk build --release` in crates/turts_client_web and use the same generic economy controls.

Changing content requires repacking and regenerated archive hashes/sizes, lockfile bytes/hash
and advertisement; server boot generates its lockfile/advertisement. Never carry old hashes
or label changed bytes as the old package release. Use a fresh version for a separately
published variant. These commands describe the future API0.2 implementation; today's API0.1
loader must reject the example. No M2 package loading or playable behavior is claimed here.
