VERDICT=fail HIGH=1

Recheck 2 of gate 2 (engine and presentation lens). Plan commit `cc514f6`, design revision 9 (`f9353bb`). In this file, "plan:N" is a line of the plan at `cc514f6`.

- F3 is fixed (plan:485, :489). I ran `proj1probe plan2` with the plugin list of plan:485, exactly as written: exit 0, at most 0.0001 px. The plant (field of view 0.9) gives exit 1, 55.90 px. This is the figure at plan:489.
- F4 mechanism is fixed (plan:481, :573, :604, :606). The capture listener and the drain at the start of the frame give `applied_seq`. On the record that holds the mouse-up, a plant one frame late shows `RingsShown` 0, so the plant can go red. I read this; I did not run it.
- Camera default is fixed (plan:483). Today's pose (`render.rs:336-342`) holds until task 11.
- `Selection` is fixed (plan:483). `CoreAdapterPlugin` sets up `Selection` with `init_resource` and writes it each frame.

NEW, High (the read time is NOT TRACED): the `frames` ring holds 64 frames, about 1 s at 60 Hz (plan:29, :481). The oracle gets `view` from the harness after `drive` (QA0 design 4.3; `run.js` does not exist yet, so the order is NOT TRACED). The SEL2 driver makes about seven gestures (plan:573). The ORD7 driver waits for the result and the move. When the oracle reads the ring, the record of the mouse-up is gone, so SEL2 and ORD7 fail on a correct build.

Fix: do one of these:
- The harness takes a copy of `turts_debug.frames` after each `io` call, and `view` gives the copies.
- The ring keeps every record whose `applied_seq` is not empty, for the whole run.
- The rule says that the oracle reads `frames` within 64 frames of the event.

## Re-check 3 (commit `488b3d3`, design revision 10)

VERDICT=pass HIGH=0

- The SEL2 and ORD7 oracle path is now sound. The mouse-up record stays in the list that `frames_clear` empties (design:1154-1155; plan:29, :481). So the oracle finds it after `drive`. On that record, a plant one frame late shows `RingsShown` 0 (or `MarkersShown` 0), so the plant goes red. I read this; I did not run it.
- Low: the plan does not say which record goes when the list reaches 4,096 (the oldest or the newest).
- Low: the record must be closed after the task 13 visuals write `RingsShown` in the same frame (a `Last` system). The plan does not state that order.
- Low: `turts_debug` is a function (`t7.js:135`). So `frames_clear` is a property of a function, and the harness calls it with its own `page.evaluate`, not with `view.bridge`.
- Low: each record holds the unit positions, so a `turts_debug()` read can be several MB with 500 units.
