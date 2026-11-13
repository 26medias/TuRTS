# Foundation contract for the first skirmish

This is the foundation scope of one playable feature. It replaces the old task sequence.
Use the existing API 0.2.4 and protocol 5 shapes. Keep unsupported features refused by the packer.

## Construction

The server accepts construction only from the owner of a declared constructor.
The construction recipe must belong to that constructor's declared building list.
Use `turts_rules::placement` for map bounds, build area, footprints, resource clearance, and queue capacity.
The client uses the same rules for its preview. The server checks the request again.
A refusal changes no balance, queue, unit, or site. It reveals no hidden blocker.

An accepted request creates one site or one held construction order within the declared caps.
Use the declared recipe duration and supported payment policy. Use the shared progress and refund rules.
Each completed site becomes exactly one building. Destruction cannot create that building later.
Record site, queue, balance, and unit changes through the existing committed tick and export paths.
The replica commits these records together. A failed delta cannot partly update the display.

## Match results and opponent

The server owns match start, defeat, and victory. Losing the declared key units causes defeat.
The surviving team wins. If both teams lose their last key units in one tick, the result is a draw.
Send the declared match record through the existing export path. The client displays that record.
Send the terminal state as one full snapshot per connection on the existing reliable channel.
Do not send unreliable deltas for that frozen tick. A missing last delta cannot hide the result.
The result screen cannot declare a winner from local unit counts.

The computer opponent submits ordinary validated orders for its own reserved slot.
It cannot mutate the world or spend another slot's balance through a separate path.
Its decisions use simulation ticks and explicit state. They do not use wall time or unseeded randomness.
Keep the opponent separate from the simulation rules so that a second human can use the same slot later.
Reserve the computer identity against network claims. The first human gets the remaining seat.
Wait for the human before starting the opponent's clock. Decide once per simulation second.
Harvest compatible resources, build one military factory, and train up to six mobile soldiers.
Send two soldiers after 75 seconds, then send another group each minute. Defend against nearby enemies.
Use content capabilities and geometry. Do not hardcode unit ids, mod ids, prices, or combat statistics.

## Contested movement

The first unit to reach a point keeps it. Later units stop nearby without overlap.
Compare arrival progress within a tick when necessary. Unit ids cannot choose a visible result.
For an exact simultaneous tie, use a symmetric stop without overlap. Do not choose a winner by id.
Changing ids alone must not change the visible arrival result.
Keep destination plans separate from physical occupancy. A moving unit cannot reserve an empty point.
Process reachable arrivals by their remaining travel time. Use geometry for collision sums and projections.
For equal arrivals near the same point, give no unit the centre. Stop their bodies on a symmetric ring.
Keep unit-id sorting only for record output. The movement kernel remains shared with audit replay.
Near the point, the later approach can clear space while the earlier body advances.
This yield is temporary. If the earlier unit leaves, the other unit resumes its original point.
An obstructed short approach uses the real movement kernel to compare travel through that obstacle.
Neutral stop points must fit inside the map and clear existing buildings.
Keep group identity while a unit yields. Queued orders and attack moves must still finish.

## Evidence required before integration

Run construction through the command path. Include a refused request, payment, completion, and destruction.
Run a full harvest, build, train, fight, and result sequence against the authoritative server.
Replay the same inputs and compare state. Repeat contested movement with the unit ids exchanged.
One independent reviewer runs the changed foundation code after implementation.
Run the cold-play probes on the complete browser build before asking the user to play.

Construction has one independent review with executed probes. Its five findings were fixed or scoped out.
The regression tests are in `crates/turts_sim/src/economy/construction.rs`,
`crates/turts_client/tests/construction.rs`, and `crates/turts_interaction/tests/placement.rs`.

Construction supports autonomous sites and the two existing upfront payment policies.
Held orders spend nothing until placement. Completion keeps the site id and existing damage.
Destruction drops the site. A killed production building releases queued reservations and loses charged costs.
Nonzero destruction refunds and construction of extractors, inserters, or conveyors stay refused.
The optional legacy audit derivation has no construction replay rule. It is parked.
The deterministic construction test replays real commands through the simulation instead.

The first browser probe built a barracks, trained a soldier, and delivered timber.
Resource selection now shows the node name and remaining amount.
The first-click probes passed at 1280x720 and 1024x768. Each checked three own units,
six order buttons, and all four extra interactions. The 1280x720 buttons probe passed.
These are partial readiness checks. The arrival rule and full readiness remain open.

Match results and the opponent have one independent review with executed probes.
It found a lost terminal delta and a map-derived seat count. Both are fixed.
The terminal wire test checks the reliable channel. The API keeps its fixed two seats.
The opponent test runs harvest, construction, training, fighting, and defeat through ordinary orders.

The arrival change has one independent review with executed probes.
Its close-arrival and illegal-stop findings are fixed. The permanent tests include those reproductions.
They also check a blocked route, an existing holder, fractional arrivals, and a cancelled approach.
The real-package command test exchanges ids and command order. The visible positions remain equal.
