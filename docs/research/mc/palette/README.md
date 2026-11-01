# VIS1 palette evidence (2026-11-01, orchestrator)

Codex stop STOP-PAL1: the Okabe-Ito default palette of design 9.1 does not keep a CIEDE2000
distance of at least 20 between each pair under protan, deutan and tritan simulation (Machado 2009,
severity 1.0), as VIS1 claims. Codex evidence: `vault/Reports/2026-11-01 MC palette stop.md`. This
folder holds the orchestrator's own runs (`colour` 0.4.6, `numpy` 1.26.4, `python3 -I`).

## Okabe-Ito minimum pair distance by severity

| Severity | protan | deutan | tritan |
| --- | ---: | ---: | ---: |
| 0.0 (no simulation) | 21.73 | 21.73 | 21.73 |
| 0.2 | 17.99 | 16.96 | 20.60 |
| 0.5 | 15.37 | 13.42 | 19.41 |
| 0.8 | 14.86 | 12.00 | 16.50 |
| 1.0 | 12.25 | 11.61 | 10.87 |

No severity above 0.2 passes 20 for deutan. A severity change cannot save the claim.

## Is a palette at 20 possible?

`optimize.py <seconds> <seed>`: simulated annealing over 8 sRGB colours, objective = the minimum
pair distance over normal vision and the three simulations at severity 1.0.

- No constraint, 240 s, seed 1: minimum 25.16. Palette `#0399FF #716502 #2E0F00 #001238 #FE6386
  #BEFE0A #FFEBFF #0000EF`. Two colours are near black and one near white.
- `optimize_l.py` (L* between 30 and 90), 240 s, seed 2: minimum 21.72 (normal 23.54, protan 21.72,
  deutan 22.20, tritan 21.94). Palette `#FFD9A8 #C78700 #42F3F1 #3400F1 #703854 #A27FFF #255C00
  #629A87`.

So 20 is possible for 8 colours, with a thin margin and with colours far from the usual team
colours. Not run on models or under the scene light: NOT TRACED.
