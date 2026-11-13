# Contested move points: arrive first (CEO, 2026-11-12)

**Status:** Decided by the CEO in chat, 2026-11-12.

**Question (slice plan section 6, touch point 1):** two units are sent to one point. Which unit keeps it?

**Decision:** the unit that REACHES the point first keeps it. A unit that arrives later stops as
close to the point as it can without overlap. CEO words: "whoever got to the point first keeps the
point, and whoever arrives later can get as close to the target point as possible without clipping
into the unit who is at the actual point"; "We do 'arrive first' as that is the natural way units
would work in real life."

**Rejected:** "ordered first" (the slice design text for A2c, gate case S12) and the old behaviour
(the unit with the lower id gives the point up, an effect of the loop order).

**Rule that follows:** the unit id must not decide a result that a player can see. It is a
tie-break only for two arrivals in the same tick. The CEO named the old id effect as technical
debt ("is not good as that implies tech debt already").

**Consequences:**
- Slice task A2c: the design is reworked and gated before the build; the CEO sees the rule text.
- Slice task A2b1: not affected; its hold is lifted.
- An audit is owed: every place where unit-id order decides a player-visible result.
- Not objected to: above 2,048 single moves in one tick, the rest start one tick later.

Source of record: `docs/superpowers/plans/2026-11-12-pre-mp1-slice.md` on branch `mp1-design`,
Binding amendments, "CEO DECISION on touch point 1".
