# M3 C1 observable capacity fault

**Status:** Decided by the orchestrator, 2026-10-28.

## Ruling

C1 is a compound semantic fault. It disables all three extractor-inventory capacity defenses:

1. the Extract phase's free-space clipping;
2. the `extract_from_node` helper's capacity check; and
3. transaction preflight's inventory-capacity check.

After the extractor first reaches `Full`, the C1 scenario runs and audits 100 more ticks. The independent conservation oracle must reject the first over-capacity state with `RES` (`used > capacity`).

## Evidence and reason

The original plant removed only the inner helper check. It stayed green because the phase clamps extraction to free space and transaction preflight checks capacity again. The original C1 scenario also stopped at the first `Full` state, before it ran a blocked retry. This could not test the intended failure.

The compound fault was compiled in a private source copy. The unchanged C1 oracle failed at `RES: inventory 1 uses 5 > 4`. Restoring both modified production files returned their exact source hashes. The existing single-helper removal remains a useful redundant-defense observation, but it is not a semantic refutation.

## Cost if wrong

The fault registry must describe three coordinated enforcement bypasses, and the C1 scenario runs 100 extra ticks. This adds test and mutation setup complexity. It does not change product behavior or weaken runtime defenses.
