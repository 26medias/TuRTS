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
The result screen cannot declare a winner from local unit counts.

The proposed computer opponent submits ordinary validated orders for its own slot.
It cannot mutate the world or spend another slot's balance through a separate path.
Its decisions use simulation ticks and explicit state. They do not use wall time or unseeded randomness.
Keep the opponent separate from the simulation rules so that a second human can use the same slot later.

## Contested movement

The first unit to reach a point keeps it. Later units stop nearby without overlap.
Compare arrival progress within a tick when necessary. Unit ids cannot choose a visible result.
For an exact simultaneous tie, use a symmetric stop without overlap. Do not choose a winner by id.
Changing ids alone must not change the visible arrival result.

## Evidence required before integration

Run construction through the command path. Include a refused request, payment, completion, and destruction.
Run a full harvest, build, train, fight, and result sequence against the authoritative server.
Replay the same inputs and compare state. Repeat contested movement with the unit ids exchanged.
One independent reviewer runs the changed foundation code after implementation.
Run the cold-play probes on the complete browser build before asking the user to play.

Implementation and the independent review have not started. This document does not claim a working skirmish.
