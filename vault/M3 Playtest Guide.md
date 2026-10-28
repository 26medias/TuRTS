# M3 playtest guide

Status: final acceptance is still running. Use [[M3 Execution Board]] for the readiness verdict. The repository README will contain launch instructions at milestone close.

Forge is an industrial demonstration with a prebuilt factory. There is no construction, AI opponent or victory screen in this milestone. The visuals use simple shapes and text panels.

## First client: the factory

The first client to join owns the factory. Left-click an owned unit to select it and inspect its status and inventory. The Bore Drill, Chain Belts, Arm Loaders and Shell Press run automatically; no Gather command is needed to start them.

Follow the chain from the Ore Vein on the left through the drill and belts to the press. Inspect the Shell Press's separate Main and Output inventories. Its shells feed the Shell Turret above it and the Proving Lab below it.

Select the Proving Lab and wait until its Main inventory has ten available shells. Use **Research Hardened Shells**. Watch its queue and completed research; the turret's damage rises from 10 to 20 after completion. Research is private to the factory owner.

Use **Disable/Enable** on eligible units to observe pauses and backpressure. **Remove** is available only on eligible units and destroys their contents. Start a fresh match after experimenting with removal if the factory can no longer supply itself.

## Second client: the drones

The second client to join owns two Target Drones. One starts in turret range and takes ten hits of 10 damage. The farther drone starts outside range.

After the factory player confirms research completion, select the farther drone in the second client and right-click a point near the turret to move it into range. It should take five hits of 20 damage. Move it only after the owner confirms completion; the second client does not receive the factory owner's research records.

The drone positions are (24,12) and (30,4); the turret is at (17,12). These are map coordinates for orientation, not screen pixels.

## Fresh matches and feedback

The browser and native clients share two permanent match slots. Closing a tab does not release its slot. Restart the server for a fresh match; do not open a third client expecting another slot.

Useful feedback includes: what you clicked, what you expected, what happened, whether you were the first or second client, and a screenshot if the selection or panels were unclear. Tell us about unclear status, invisible stock movement, controls that seem unresponsive, or a factory that stops without an explanation.
