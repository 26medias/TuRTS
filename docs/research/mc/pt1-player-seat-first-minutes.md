# PT1: the player seat on the first minutes of forge (from screenshots only)

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=UNPLAYABLE

Note: I use the format you asked for. In my own four-part check, this scene has no goal and no win or lose. My usual line 1 is `VERDICT=needs-decision`, because this is a test scene and not a game. I only looked at pictures. I did not hear the sound, so every point about sound is NOT TRACED. The full play log is in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/player-playlog-run135.txt

**Items 1 to 4**

1. Click to play
    - What I see: a dark screen with a wall of text. "Click to play" sits on top of the words "Box select: Left click".
    - What I think the game is: a factory game, maybe.
    - What I would do next: click.
    - Where I would give up: not here yet.
2. Guide open
    - What I see: the same wall of text, with a "Close" button. The button covers the Alerts box.
    - What I think: this is a manual, not a game.
    - What I would do next: press Close.
    - Where I would give up: I would not read it.
3. Guide pages
    - What I see: about 50 key lines. 27 of them are "Add to group / Assign group / Select group 1–9".
    - What I would do next: scroll, then close the guide.
    - Where I would give up: I would start to skim here.
4. Guide closed
    - What I see: a small green board, some blue blobs, a row of blue dots, a tan box and two orange dots. Nothing has a name. One alert: "A loader is waiting for an item".
    - What I think: something is broken, and I am meant to fix it.
    - What I would do next: click a blob.
    - Where I would give up: **here**. Nothing tells me what to do.

**A. The first 60 seconds**

The pictures do not show seconds, so this is the order of events only.

- 0 s: I click to play. The guide stays open.
- About 5 s: I scroll the key list.
- About 15 s: an alert comes up while the guide is still open. A beep starts (NOT TRACED).
- About 20 s: I press Close. I see a board with no names, no goal and no money. The alerts pile up: "loader…", "A turret needs more shells".
- About 30 s: I move the mouse over a blob and see "Bore Drill · Slot 1". It is blue, like all the others.
- About 40 s: I click a blob and five panels open.

The first moment with no next step is the screen with the guide closed (pt1-s03-guide-closed.png).

**B. The guide**

- Words that help:
    - "The drill lifts ore. A belt carries it to the press, which turns it into shells."
    - "Research Hardened Shells there to improve your turret."
- Words that do not help:
    - "Use a group action to assign, add or recall units."
    - "Type modifier: ControlLeft, ControlRight"
    - The 27 group lines.
    - "Move shells into the proving lab". No order on the screen moves shells.
- It is too long to read before the first action.
- The missing sentence: "Your goal: make your turret stronger. First, click the Proving Lab (the blob at top centre)."

**C. The screen with the guide closed**

- Nothing invites a click. Mine and the enemy's look the same, because every machine is the same blue blob.
- Who owns what is confusing. The tooltip says "Shell Press · Slot 1". The panel says "Owner: Slot 0".
- What the panels mean to me:
    - "1:0 2:0 … 9:0": nothing.
    - Selected shows "HP" with no number.
    - The info panel shows "Main inventory 0 available (capacity 10)" and "Inventory: []". These read like computer text.
    - Working is empty most of the time.
    - Orders shows Disable and Enable at the same time. I cannot tell which state the machine is in.
- What looks broken or empty:
    - The Alerts box has gaps and dotted marks over its lines.
    - The research row is cut: "Hardened Shells —".

**D. The alerts**

- "A loader is waiting for an item": I do not know what a loader is. Nothing on the screen has that name.
- "A turret needs more shells": I cannot tell which turret, or what I should do about it.
- No alert tells me what to do, and I cannot click an alert to go to the problem.
- A beep each second tells me "you are failing" non-stop, and I cannot see why. I would mute the game or close the tab.

**E. The five goals**

1. Name of each machine: I can do it only by hovering over each blob. It takes about 1 minute. The sheet says "machine", but the screen shows names like "Bore Drill".
2. Research the turret upgrade: the upgrade sits in the lab, not on the turret, and it is called "Hardened Shells". It takes 2–5 minutes, mostly waiting for "0/10" to fill.
3. Check that the turret is stronger: I cannot. The turret shows "Damage: 20" with no old value and no "upgraded" mark.
4. Pause and start again: I find D and E, but after D the panel still says "Make Shells 0.2s remaining". The sheet says "pause", but the screen says "Disable" and "Enable".
5. Why an order is refused: "Shell Turret: Cannot move" shows in small yellow text far from my click, with no reason. The sheet says "refused", but the screen says "Cannot move".

**F. Changes**

(1) This test scene:

1. When the guide closes, show this in the centre: "Goal: research Hardened Shells to make your turret hit harder. Click the Proving Lab." Put a pulsing ring on the lab.
2. Show a name label over each machine at all times.
3. Show each alert once, and make it clickable. Play the sound once only. Use words like "Shell Press is out of ore: the drill is off".
4. Show a large "OFF" on a disabled machine, and show only the button that applies.
5. On the turret, show "Damage: 20 → 30 (Hardened Shells)" or the real values. Change the refused message to "Turrets cannot move. They are fixed."

(2) The first real game, from its first second:

1. One line at the top: "Destroy the enemy Command Centre."
2. One first action: "Select your Harvester and right-click the ore."
3. Clear team colours from the first frame.
4. Show money, and have it rise when the harvester comes back.
5. A warning before a loss: "Your base is under attack."

**G. Is there a game here?**

No. This is a set of checks on a board. There is nothing to want, nothing to beat, and no way to win or lose.

**Files I read**

- /home/julien/Projects/TuRTS/.claude/agents/player.md
- In /home/julien/Projects/TuRTS/target/mc-evidence/ux-run135-rows-v1/runs/run-20261112T004026Z-6OgQ2t/:
    - pt1-s02-click-to-play.png, pt1-s02-guide-open.png, pt1-s02-controls-01.png, pt1-s02-controls-02.png, pt1-s02-controls-05.png
    - pt1-s03-guide-closed.png, pt1-s04-before-pan.png, pt1-s04-hover-03.png, pt1-s04-tooltip-02.png
    - pt1-s05-lab-01.png, pt1-s05-research-tooltip-02.png
    - pt1-s06-press-01.png, pt1-s06-disabled-10.png, pt1-s07-research-ready.png, pt1-s08-turret-01.png, pt1-s09-cannot-move-02.png
    - harness-shot-59, 200, 345, 384, 655, 672 and 695 (all .png)
- I also listed the folder. I did not read controls-00, 03 or 04, and I read no JSON files.

**Other points**

- pt1-s04-before-pan.png already shows "Cannot move", so it may come from a later step than its name says. Because of that, I could not see the turret's numbers before the upgrade.

What I changed: none in the repository. The play log is in the scratchpad.
