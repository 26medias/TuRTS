---
type: report
date: 2026-11-12
tags: [turts, mc, pt1, pt2, ceo]
---

# MC play test: instructions for the CEO

You play two short tests: PT1 on the mod `forge` (two browser windows) and PT2 on the mod `muster` (one window). Together they take 15 to 30 minutes. The play log records "not blind", because you know the project.

## 1. Start the game

Open a terminal and type:

```
bash /home/julien/Projects/TuRTS-mc-play/play.sh forge
```

Wait for a line that starts with `URL:`. Keep the terminal open; Ctrl+C stops the game.

Open that address in a Chrome window that has WebGPU turned on. On Linux, Chrome has WebGPU off by default, and the game then stops with "Unable to find a GPU" in the console. Type this in a second terminal, with the address from the `URL:` line:

```
google-chrome --user-data-dir=/tmp/turts-play-profile --enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan "PASTE THE URL HERE"
```

The `--user-data-dir` value makes a separate, empty Chrome profile for the play test, so your own Chrome does not change and the flags apply. For a second window, press Ctrl+N in that Chrome. If the error stays, open `chrome://gpu` there and give Root the "WebGPU" and "Vulkan" lines.

- The first start after an update compiles for some minutes. Root made one full build on 2026-11-11 (about 25 minutes), so the next start is fast if the code did not change.
- The address has new port numbers at each start. Always use the address of the current start.
- For PT2, stop the first game (Ctrl+C) and type: `bash /home/julien/Projects/TuRTS-mc-play/play.sh muster`
- The play folder is at the integration branch head of 2026-11-11 (commit 8d6902c: Tasks 21 and 22, and the UX repairs of ruling O15). To play a later build, ask Root to update the folder.

## 2. PT1 on `forge`: two windows

Use the mouse and the keyboard only. Window A is the factory player. Window B is the drone player.

1. Open the address in window A. Play the window A goals.
2. About 30 seconds after window A started, open the SAME address in a second Chrome window (window B), side by side.

Window A goals:
- Find the name of each machine.
- Research the turret upgrade.
- Check that the turret is stronger.
- Pause one machine and start it again.
- Find out why an order is refused.

Window B goals:
- Find the enemy turret and its range.
- Make your drone control group 1, and centre the view on it with the group key.
- In the menu, make the view scroll faster and change the Stop key.
- Reload the page, then check that group 1 and the new Stop key still work.
- Send your drone to two points, one after the other, with one order, and stop it with the new key.
- Send your drone into the turret range and see the result.

## 3. PT2 on `muster`: one window

- Select all 40 troopers with one box and send them to one point. Say if they stop near the marker, apart, and without jitter.
- Send 20 troopers to a point and let them stop. Then send the other 20 to the same point. Say what you see. (Ruling O14: in MC, one or more units of the second group can stop early with a "blocked" reason. M4b owns the full repair. Your opinion on how it looks is the point of this goal.)

## 4. When does a test fail

A test fails when you cannot find an object, an action or a reason, or when 60 seconds pass with no sign of progress. That is a result, not your error: write where it happened.

## 5. What to report

Tell Root in the session, or write under "Your notes" below. For each goal: done, or where you stopped and why. Also anything that looks wrong, slow, or not clear. Then say "PT1 signed off" and "PT2 signed off", or what must change first.

## 6. Known items (you do not need to report these)

- The selection row reads "Proving Lab x1 - HP" with no number: that unit has no health value, and the `forge` label expects one (MP1 list).
- Small marks above the first alert row, and wide gaps between alert rows (MP1 list).
- The settings menu uses a different typeface from the game panels.
- With no GPU, or in a browser that blocks the GPU, the game is very slow. Play on this desktop in Chrome.
- If the browser does not start the sound, a notice "Sound is off. Click or press a key to turn it on." shows for 8 seconds; the game is playable.

## Your notes

(write here)
