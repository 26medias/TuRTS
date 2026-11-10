#!/usr/bin/env python3
"""Build small Forge unit portraits from local vector primitives (Python stdlib only)."""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parent / "images"
SCALE = 2
SIZE = 128
S = SIZE * SCALE
TRANSPARENT = (0, 0, 0, 0)
BG = (16, 24, 32, 255)
PANEL = (28, 42, 53, 255)
EDGE = (91, 117, 132, 255)
LIGHT = (217, 226, 230, 255)
BLUE = (0, 114, 178, 255)
ORANGE = (230, 159, 0, 255)
CYAN = (86, 180, 233, 255)
GREEN = (0, 158, 115, 255)


class Canvas:
    def __init__(self) -> None:
        self.pixels = [TRANSPARENT] * (S * S)

    def put(self, x: int, y: int, colour: tuple[int, int, int, int]) -> None:
        if 0 <= x < S and 0 <= y < S:
            self.pixels[y * S + x] = colour

    def rect(self, x0: float, y0: float, x1: float, y1: float,
             colour: tuple[int, int, int, int]) -> None:
        x0, y0, x1, y1 = (round(value * SCALE) for value in (x0, y0, x1, y1))
        for y in range(max(0, y0), min(S, y1)):
            for x in range(max(0, x0), min(S, x1)):
                self.put(x, y, colour)

    def circle(self, cx: float, cy: float, radius: float,
               colour: tuple[int, int, int, int]) -> None:
        cx, cy, radius = cx * SCALE, cy * SCALE, radius * SCALE
        left, right = max(0, int(cx - radius)), min(S, int(cx + radius + 1))
        top, bottom = max(0, int(cy - radius)), min(S, int(cy + radius + 1))
        radius_squared = radius * radius
        for y in range(top, bottom):
            for x in range(left, right):
                if (x - cx) ** 2 + (y - cy) ** 2 <= radius_squared:
                    self.put(x, y, colour)

    def polygon(self, points: list[tuple[float, float]],
                colour: tuple[int, int, int, int]) -> None:
        points = [(round(x * SCALE), round(y * SCALE)) for x, y in points]
        min_y = max(0, min(y for _, y in points))
        max_y = min(S - 1, max(y for _, y in points))
        for y in range(min_y, max_y + 1):
            crossings = []
            for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1]):
                if y0 == y1 or y < min(y0, y1) or y >= max(y0, y1):
                    continue
                crossings.append(round(x0 + (y - y0) * (x1 - x0) / (y1 - y0)))
            crossings.sort()
            for left, right in zip(crossings[::2], crossings[1::2]):
                for x in range(max(0, left), min(S, right + 1)):
                    self.put(x, y, colour)

    def line(self, a: tuple[float, float], b: tuple[float, float], width: float,
             colour: tuple[int, int, int, int]) -> None:
        ax, ay = a[0] * SCALE, a[1] * SCALE
        bx, by = b[0] * SCALE, b[1] * SCALE
        steps = max(1, round(math.hypot(bx - ax, by - ay)))
        radius = width * SCALE / 2
        for step in range(steps + 1):
            ratio = step / steps
            self.circle((ax + (bx - ax) * ratio) / SCALE,
                        (ay + (by - ay) * ratio) / SCALE,
                        radius / SCALE, colour)

    def finish(self, path: Path) -> None:
        rows = []
        for y in range(SIZE):
            row = bytearray([0])
            for x in range(SIZE):
                samples = [
                    self.pixels[(y * SCALE + dy) * S + x * SCALE + dx]
                    for dy in range(SCALE) for dx in range(SCALE)
                ]
                row.extend(round(sum(pixel[channel] for pixel in samples) / len(samples))
                           for channel in range(4))
            rows.append(bytes(row))
        payload = b"".join(rows)

        def chunk(kind: bytes, data: bytes) -> bytes:
            body = kind + data
            return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

        png = (b"\x89PNG\r\n\x1a\n" +
               chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)) +
               chunk(b"IDAT", zlib.compress(payload, level=9)) + chunk(b"IEND", b""))
        path.write_bytes(png)


def badge() -> Canvas:
    image = Canvas()
    image.circle(64, 64, 61, EDGE)
    image.circle(64, 64, 57, BG)
    image.circle(64, 64, 48, PANEL)
    return image


def portraits() -> dict[str, Canvas]:
    result = {}

    image = badge()
    image.rect(43, 36, 85, 78, LIGHT)
    image.rect(38, 74, 90, 87, BLUE)
    image.polygon([(54, 77), (75, 77), (68, 106), (61, 106)], ORANGE)
    image.circle(64, 43, 9, CYAN)
    result["portrait_drill.png"] = image

    image = badge()
    image.rect(25, 48, 103, 80, BLUE)
    for x in range(31, 100, 17):
        image.rect(x, 53, x + 10, 74, LIGHT)
    image.rect(31, 84, 97, 92, ORANGE)
    result["portrait_belt.png"] = image

    image = badge()
    image.rect(37, 83, 91, 96, BLUE)
    image.circle(64, 79, 13, LIGHT)
    image.line((62, 74), (86, 48), 9, LIGHT)
    image.line((84, 48), (99, 52), 7, ORANGE)
    image.circle(40, 94, 5, CYAN)
    result["portrait_arm_loader.png"] = image

    image = badge()
    image.rect(34, 49, 94, 94, BLUE)
    image.rect(43, 36, 85, 52, LIGHT)
    image.rect(56, 40, 72, 56, ORANGE)
    image.rect(47, 62, 81, 72, CYAN)
    image.rect(43, 79, 85, 86, LIGHT)
    result["portrait_press.png"] = image

    image = badge()
    image.polygon([(35, 80), (44, 58), (84, 58), (94, 80), (84, 96), (44, 96)], LIGHT)
    image.rect(59, 48, 94, 61, ORANGE)
    image.rect(85, 50, 102, 58, CYAN)
    image.circle(64, 79, 10, BLUE)
    result["portrait_turret.png"] = image

    image = badge()
    image.rect(32, 38, 96, 96, BLUE)
    image.rect(43, 48, 58, 68, CYAN)
    image.rect(70, 48, 85, 68, CYAN)
    image.circle(64, 78, 14, ORANGE)
    image.rect(57, 91, 71, 104, LIGHT)
    result["portrait_lab.png"] = image

    image = badge()
    image.polygon([(64, 30), (79, 67), (101, 87), (78, 84), (64, 99),
                   (50, 84), (27, 87), (49, 67)], BLUE)
    image.circle(64, 68, 10, LIGHT)
    image.polygon([(64, 56), (73, 70), (64, 78), (55, 70)], ORANGE)
    image.circle(64, 68, 3, CYAN)
    result["portrait_drone.png"] = image
    return result


if __name__ == "__main__":
    ROOT.mkdir(parents=True, exist_ok=True)
    for name, image in portraits().items():
        image.finish(ROOT / name)
