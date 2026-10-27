---
type: decision
date: 2026-10-27
tags:
  - turts
  - decision
  - m2
---

# M2 camera framing around the real panel

**Status:** Approved by the orchestrator for Task 16 recovery.

At 1280×720 the active full-window camera projects the base owner-0 Command Center and Harvester under the left economy panel. The world hit test and panel pointer suppression are correct; changing click coordinates or bypassing UI suppression would hide the defect.

Keep one full-window perspective camera and the shared Bevy UI target. After Bevy UI layout, read the actual `UiPanelRoot` `ComputedNode` and `UiGlobalTransform`, the primary window's physical dimensions, the accepted map dimensions, and maximum declared unit/node pick radius. Fit all four radius-padded map corners to the unobstructed rectangle by solving the perspective camera aim and distance from the active projection FOV. Refit after layout/window changes and before transform propagation.

When the measured left panel consumes over one third of the viewport width, make it a nearly full-width bottom anchored scroll panel with capped height; frame the map above its measured top edge. Keep actual controls and pointer suppression. This scope adds no zoom, pan, fixture-specific placement, or world hit-test changes.

The initial regression measured owner-0 Command Center center `(316.11, 100.88)` beneath the panel ending at x=402. Its red test evidence is persisted in `target/m2-export/task16-evidence/red/camera-accessibility-red.log`. Final regression and browser acceptance must use catalog-derived radii and actual computed panel geometry.
