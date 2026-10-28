---
type: decision
date: 2026-10-27
status: approved
tags: [turts, m3, testing, privacy]
---

# M3 F11 private research trigger

The user chose: keep research private; let the test coordinator trigger the opponent's move after the owner's replica confirms completion.

The original SPEC 12.2 kept research owner-only, while F11 required the opponent's replica to see that private completion. The two rules could not both hold. F11 and plan T15b/T16 now preserve owner-only export. The owner client supplies a live report of its validated replica; the test coordinator releases the opponent move only after this report confirms completion. The move applied tick must remain later than the completion tick. Slot 1 runs from the beginning so its public health trace includes all hits.

The exact test-only report and signal interface must be fixed before T15b. No product-facing research broadcast is authorized. No source implementation change has been made for this decision.
