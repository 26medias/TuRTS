

## C386: the PT1 oracle judges from harness records only; repair every RUN115 finding before runtime (2026-11-11)

Conflict: RUN115 M2: pt1-oracle.js reads files that only pt1-driver.js writes (pt1-oracle.js:146,344,364) and screenshot names that only the driver uses. Plan Task 23 step 2 says the player agent plays through the harness io with the goals only and "the oracle checks the 19 Player path steps". The agent does not run the driver, so the oracle as written cannot judge that run.

Choice: option (a). The PT1 oracle takes its evidence from what the harness records for any player of the io contract: the input log, the scene checkpoints and screenshots in capture order, and the server files. It does not depend on a driver-written file or a driver-chosen name; a driver label may only be an optional hint. The person's play is judged by the play log and the sign-off (plan Task 23 step 3), not by this oracle. Also, by design 13.3, the M3 pair does disable, enable and remove through the forge HUD buttons (RUN115 L1), and all RUN115 findings High to Low are repaired now, because a repair before runtime is cheaper than a headed red.

Evidence: RUN115 reply; plan lines 705-713; brief section 4 lines 49 and 53.

Alternative not taken: (b) the oracle judges the scripted run only. It leaves plan Task 23 step 2 with no instrument.

Global116/170 after RUN116 registration.
