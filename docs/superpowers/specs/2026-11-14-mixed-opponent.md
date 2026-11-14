# A mixed computer army

The computer trains Swordsmen and Crossbowmen through normal paid orders.
It first replaces a missing gatherer. It then trains the least represented available fighting type.
Queued jobs count toward each type's population. The package recipe order resolves equal counts.
Keep the existing economy, six-unit army cap, opening delay, and attack waves.
Keep the current authority, protocol, simulation, and public mod API contracts.
This is a small opponent policy change. It does not complete the MP2 bot framework.

| Row | Check |
| --- | --- |
| BOT1 | An ordinary skirmish opponent builds Barracks, harvests, trains both fighting types, and defeats an idle player. |
| BOT2 | The opponent uses paid production orders. Its mixed army costs more than its starting balance. It needs timber deliveries. |
| BOT3 | Repeating the same human join produces the same complete audited simulation state. No commands run before a human joins. |
| BOT4 | Browser games with normal stats reach Victory and Defeat. New skirmish starts a fresh match. Final cold probes pass. |

Every test browser uses a fresh profile and --alsa-output-device=null.
Do not merge or push main.
