"""Generate independent whole-number expectations with Python Fraction. No Rust calls."""
from fractions import Fraction as F
from pathlib import Path

M = 2**64 - 1
U = 2**128 - 1
# base, bonus, cell num/den, curve (0 none, 1 ratio, 2 percent), curve num/den,
# cap num/den, points, flat, minimum, health. Include wide products and checked edges.
rows = []
for base in [0, 1, 10, 15, 100, M]:
    for cell in [(1, 1), (1, 2), (3, 4), (0, 1)]:
        rows.append((base, 0, *cell, 0, 1, 1, 0, 1, 0, 3, 1, M))
for points in [0, 1, 5, 100, 200, 300, M]:
    for curve in [1, 2]:
        rows.append((100, 7, 3, 4, curve, 2, 1, 3, 4, points, 2, 1, 1000))
rows += [
    (15, 0, 1, 2, 1, 2, 1, 0, 1, 1, 0, 0, 100),
    (10, 0, 1, 2, 0, 1, 1, 0, 1, 0, 3, 1, 100),
    (100, 7, 1, 2, 1, 50, 3, 0, 1, 5, 3, 1, 2),
    (M, 1, 1, 1, 0, 1, 1, 0, 1, 0, 0, 1, M),
    (M, 0, M, M, 1, M, M, 0, 1, M, 0, 0, M),
    (M, 0, M, 1, 1, M, 1, 0, 1, 1, M, 0, M),
    (M, 0, M, 1, 2, 0, 1, 0, 1, 0, M, 0, M),
    (10, 0, 1, 1, 2, M, 1, M-1, M, M, 0, 0, 100),
]
lines = ['# base bonus cell_num cell_den curve curve_num curve_den cap_num cap_den points flat minimum health expected_hit expected_applied']
for row in rows:
    base, bonus, cn, cd, curve, n, d, capn, capd, points, flat, minimum, health = row
    factor = F(1)
    overflow = base + bonus > M
    if curve == 1:
        curve_den = n + d * points
        overflow |= curve_den > U or cd * curve_den > U
        factor = F(n, curve_den)
    elif curve == 2:
        overflow |= n * points * capd > U
        factor = 1 - min(F(capn, capd), F(n, d) * points)
    raw = F(base + bonus) * F(cn, cd) * factor
    hit = max(minimum, min(M, max(0, raw.numerator // raw.denominator - flat)))
    expected = ['overflow', 'overflow'] if overflow else [str(hit), str(min(hit, health))]
    lines.append(' '.join(map(str, row)) + ' ' + ' '.join(expected))
path = Path(__file__).resolve().parents[3] / 'crates/turts_rules/tests/fixtures/damage_pipeline.tsv'
path.write_text('\n'.join(lines) + '\n')
print(f'{len(rows)} independent cases: {path}')
